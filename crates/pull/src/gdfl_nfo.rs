//! GDFL NSE options (NFO) tick files: the day, the contract a file name
//! states and every row, read from the vendor's yearly zips or from the
//! verified tick store and checked before anything folds them. D-2806.
//!
//! # What this reads
//!
//! The operator's GDFL options history is one outer zip per year holding one
//! STORED day zip per trading day, `<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.zip`,
//! and the verified tick store keeps every entry of each of those day zips in
//! `options/<yyyy>/<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.bts` (tick store
//! FORMAT.md v1, §2, recorded in `docs/00-charter.md`). Inside a day the
//! entries are `GFDLNFO_TICK_<ddmmyyyy>\Options\<TICKER>.NFO.csv`, with the
//! Windows separator kept byte for byte (FORMAT.md §1). Measured on the
//! operator's zips (charter, GDFL options section): the day zips hold ONLY an
//! `Options` folder, so there are no futures in this data and no `-I`
//! continuous-futures name to resolve.
//!
//! Every row has the capital-market header
//! `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`,
//! the ticker being the file stem (`<TICKER>.NFO`). Unlike the capital-market
//! reader this one keeps the open interest: an option bar carries it.
//!
//! # The contract a ticker names (FORMAT.md §6, revised 2026-10-04; D-2806)
//!
//! Two vendor forms: the DAY form `UNDERLYING DD MON YY STRIKE CE|PE`
//! (`BANKNIFTY24FEB2651500PE`) and the MONTHLY form `UNDERLYING YY MON STRIKE
//! CE|PE` (`ACC18OCT1280PE`, October 2018). They collide (`ADANIENT18DEC195CE`
//! reads as Dec 2018 strike 195 or as 18 Dec 2019 strike 5, and from 2020 on
//! every day-form name also parses as a monthly one), so only the era keeps
//! them apart, by the trade date:
//!
//! 1. on or after [`DAY_FORM_FROM`] (2019-02-01): the day form only;
//! 2. before it: the day form only for the index weeklies NIFTY and
//!    BANKNIFTY (any weekday, which covers a holiday-shifted Tuesday expiry),
//!    and the monthly form for every option, index monthlies included;
//! 3. the expiry lies from the trade date (a monthly: its month from the
//!    trade month) to the trade date plus 2,200 days, and the strike is > 0;
//! 4. no reading, or more than one, is refused by name, never guessed.
//!
//! A monthly name states no expiry DAY. It is derived from NSE's own rule
//! for equity derivatives, "Last Thursday of the expiry month. If the last
//! Thursday is a trading holiday, then the expiry day is the previous trading
//! day" (NSE contract specifications, archived 2019-08-16; charter GDFL
//! options section), walked back on [`crate::calendar::sourced_kind_of`]. A day the
//! calendar has not measured refuses (`MonthlyExpiryUnmeasured`), and a file
//! traded AFTER its derived expiry refuses (`ExpiryPassed`): that is the
//! per-day half of the cross-check against the vendor's own appearances.
//!
//! The strike must be written canonically (no leading zero, no trailing
//! fractional zero, at most two places), so every name that decodes renders
//! back to itself byte for byte ([`OptionTicker::ticker_text`]). The
//! underlying may hold any byte a `Symbol` admits, digits included (`M&M`,
//! `NAM-INDIA`, `360ONE`, `TV18BRDCST`).
//!
//! # The expiry a contract SETTLED on, not the one its text states (D-3331)
//!
//! Until D-3331 an expiry the day form stated on an exchange holiday was kept
//! as stated, the name being the contract's identity. That split one contract
//! into two or three store series whenever NSE re-dated it and the vendor
//! renamed the file (DB-018, DB-G50), and measured tenor to a closed day
//! (DB-G51). Every reading is now settled by [`crate::expiry_alias::settle`]:
//! a measured re-dating row, or the calendar (a closed stated day settles on
//! the previous open session, the charter's NSE rule). The contract segment
//! carries the SETTLED expiry, [`OptionTicker::stated`] keeps the vendor's
//! text as the alias, and [`OptionTicker::settlement`] says how. A closed
//! stated day with no open session the calendar records before it, a retired
//! text traded after its rename, and a trade after the settled day are
//! refused by name ([`NfoRefusal::ExpiryUnsettled`]).
//!
//! # A strike the day's ladder makes impossible (D-3331)
//!
//! A decoded strike more than [`STRIKE_PLAUSIBLE_MULTIPLE`] times the median
//! strike of the same underlying's names that day is refused by name
//! ([`NfoRefusal::StrikeImplausible`]) when the file is read, and so is never
//! stored or listed (DB-027, DB-G52). The median is the day's own, computed
//! once per listing ([`NfoDay::strike_median`]).
//!
//! # What it hands on, and what it does not decide
//!
//! [`decode`] returns every row in FILE ORDER. It never sorts, never drops a
//! row and never re-stamps one: placing a late row and dropping LTQ = 0 rows
//! are the import's rules ([`crate::gdfl_import`], D-2802). It refuses, by
//! name, bytes that are not the file they claim to be.
//!
//! # Cost
//!
//! A yearly zip's central directory is read once per [`NfoZips`] and year,
//! O(entries in the year zip), and a day is then one hash probe into it
//! (WP-16). A day listing is O(entries) once per day; after it [`NfoDay::locate`] is
//! one hash probe (expected O(1), measured by `C-GI-01` in
//! `crates/pull/benches/ratio.rs`). [`decode_ticker`] is O(ticker length),
//! bounded by the 64-byte cap it refuses past, plus one settlement (bounded
//! walks over compile-time tables). The day's strike medians are
//! O(entries · log entries) once per listing, on its first read; after that
//! the strike check is one hash probe. [`decode`] is O(bytes).
//! Fetching a file is O(its compressed and rebuilt bytes). The listing and
//! the fetch are limits in `docs/06-limits.md`, not constant-time claims.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;

use crate::expiry_alias::{SettleRefusal, Settlement, settle};
use crate::gdfl_archive::{
    MONTHS, ReadAt, ZipEntry, ZipLocator, member_bytes, stored_span, zip_entries,
};
use crate::gdfl_cm::{CmRefusal, HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST_SPACED, ListedFile};
use crate::gdfl_tickstore::{IndexEntry, read_index, rebuild};
use crate::session::Day;

/// The day folder's prefix, before its `ddmmyyyy`.
pub const FOLDER_PREFIX: &str = "GFDLNFO_TICK_";

/// The one sub-folder of a day that holds option files.
pub const OPTIONS_FOLDER: &str = "Options";

/// The file name's suffix after the ticker, in the two spellings the
/// capital-market reader also admits for its extension.
const NFO_SUFFIXES: [&str; 2] = [".NFO.csv", ".NFO.CSV"];

/// The ticker's own suffix on every row.
const ROW_SUFFIX: &str = ".NFO";

/// The longest ticker read. The longest measured is far below it (charter,
/// GDFL options section); a longer one is refused before it is scanned, which
/// is what keeps [`decode_ticker`] bounded.
pub const TICKER_CAP: usize = 64;

/// How far after the trade date a stated expiry may lie (FORMAT.md §6).
pub const EXPIRY_HORIZON_DAYS: u32 = 2_200;

/// How many times the day's median strike of an underlying a strike may be
/// before it is refused as impossible ([`NfoRefusal::StrikeImplausible`],
/// D-3331: `k` = 10).
///
/// **Chosen from a measurement, not sourced** (D-3331 decision 2). Measured
/// 2026-10-09 by the ignored test
/// `the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes`
/// over the GDFL options name census (names only, from each day zip's central
/// directory: 1,997 vendor days from 2018-09-03 to 2026-09-24, 20,825,673
/// file-days), each underlying-day's highest admitted strike against the
/// lower median of that underlying's strikes that day: at most 5.75
/// (RELCAPITAL, 2019-08-27, during its collapse), then 5.20 (INFIBEAM,
/// 2018-11-28) and 5.00 (JPASSOCIAT, 2018-11-21), the audit's 5.75 (DB-G52).
/// The only names above it are the three NIFTY `25JUN20` strikes of
/// 2019-05-30, at about 163.5 times that day's median of 11,700. Ten clears
/// the real maximum by more than half again and sits sixteen times under the
/// impossible ones. Only the high side is refused: real ladders reach 20.0
/// times BELOW their median (INFIBEAM, 2018-10-16, the same census), so a
/// low-side bound at this `k` would refuse real contracts.
pub const STRIKE_PLAUSIBLE_MULTIPLE: i64 = 10;

/// The LOWER median of a day's strikes of one underlying, paisa, or `None`
/// for an empty ladder: the element at `(len − 1) / 2` once sorted. Sorts
/// `strikes` in place.
///
/// The one spelling of D-3331's reference strike: the import's
/// [`NfoDay::strike_median`] (a vendor day's decodable names) and the store
/// re-key's ladder (`crate::gdfl_rekey`, the stored keys with a bar that day)
/// both ask it, so the two can never disagree about what "the day's median"
/// means. O(n log n).
#[must_use]
pub fn lower_median(strikes: &mut [i64]) -> Option<i64> {
    strikes.sort_unstable();
    strikes.get(strikes.len().saturating_sub(1) / 2).copied()
}

/// Whether `strike` is more than [`STRIKE_PLAUSIBLE_MULTIPLE`] times the
/// day's `median` (D-3331 decision 2): the one comparison behind every
/// [`NfoRefusal::StrikeImplausible`], at import and at re-key. Exactly `k`
/// times the median is admitted; one paisa more is not. O(1).
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
#[must_use]
pub const fn strike_implausible(strike: i64, median: i64) -> bool {
    strike > median.saturating_mul(STRIKE_PLAUSIBLE_MULTIPLE)
}

/// Why an options file, its name or a row was refused. Every refusal names
/// itself; none is skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NfoRefusal {
    /// The ticker reads as neither vendor format.
    TickerUnparsed {
        /// The ticker.
        ticker: String,
    },
    /// The ticker has more than one reading the era admits.
    TickerAmbiguous {
        /// The ticker.
        ticker: String,
    },
    /// Two tickers of one day name one contract (a day-form and a monthly
    /// name of one index monthly, say), so their bars would file under one
    /// path: both refused.
    ContractNamedTwice {
        /// The ticker.
        ticker: String,
    },
    /// A monthly name whose derived expiry needs a day the calendar has not
    /// measured.
    MonthlyExpiryUnmeasured {
        /// The ticker.
        ticker: String,
    },
    /// A monthly name traded after its derived expiry: the rule and the
    /// vendor disagree, so neither is believed.
    ExpiryPassed {
        /// The ticker.
        ticker: String,
    },
    /// The stated expiry could not be settled (D-3331): a closed day with no
    /// open session before it, a retired text traded after its rename, or a
    /// trade after the settled day.
    ExpiryUnsettled {
        /// The ticker.
        ticker: String,
        /// Why, in the settlement's own words.
        why: SettleRefusal,
    },
    /// The strike is more than [`STRIKE_PLAUSIBLE_MULTIPLE`] times the day's
    /// median strike of its underlying (D-3331): a name no real contract
    /// carries, refused rather than stored and listed.
    StrikeImplausible {
        /// The ticker.
        ticker: String,
        /// The decoded strike, paisa.
        strike: i64,
        /// The day's median strike of the underlying, paisa.
        median: i64,
    },
    /// The underlying is not a symbol the store can file.
    UnderlyingRefused {
        /// The ticker.
        ticker: String,
    },
    /// The file is named for a symbol the exchange was not using on the trade
    /// day: an old symbol on or after its dated change, or a new one before
    /// it (`crate::gdfl_lineage::filed_underlying`; DB-B05, DB-B11, DB-B62).
    UnderlyingStale {
        /// The ticker.
        ticker: String,
        /// The dated change the name is on the wrong side of.
        why: brutex_core::alias::Stale,
    },
    /// The contract does not render into the store's contract segment.
    ContractUnrenderable {
        /// The ticker.
        ticker: String,
    },
    /// Two entries of one day name one ticker.
    DuplicateTicker {
        /// The ticker.
        ticker: String,
    },
    /// The first line is neither observed header.
    HeaderUnknown,
    /// The Ticker field is not the file's own stem.
    TickerMismatch {
        /// One-based line number.
        line: u32,
    },
    /// The Date field is not the day's date as `DD/MM/YYYY`.
    DateMismatch {
        /// One-based line number.
        line: u32,
    },
    /// A row that is not ten well-formed fields.
    MalformedRow {
        /// One-based line number.
        line: u32,
    },
    /// An LTP that is not a plain decimal of at most two places, or a traded
    /// row (LTQ > 0) priced at zero.
    PriceRefused {
        /// One-based line number.
        line: u32,
    },
    /// An open interest past `i64::MAX`.
    OpenInterestRefused {
        /// One-based line number.
        line: u32,
    },
    /// More rows than [`crate::fetch::MAX_ROWS`].
    RowsOverCap {
        /// The cap.
        cap: usize,
    },
    /// The source or the byte checks refused, in the reader's own words.
    Source(CmRefusal),
}

impl core::fmt::Display for NfoRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TickerUnparsed { ticker } => {
                write!(
                    f,
                    "{ticker}: the ticker reads as neither GDFL option format"
                )
            }
            Self::TickerAmbiguous { ticker } => write!(
                f,
                "{ticker}: more than one reading of the ticker; refused rather than guessed"
            ),
            Self::ContractNamedTwice { ticker } => write!(
                f,
                "{ticker}: another ticker of the same day names the same contract; both refused rather than merged"
            ),
            Self::MonthlyExpiryUnmeasured { ticker } => write!(
                f,
                "{ticker}: the monthly expiry day needs a day the calendar has not measured"
            ),
            Self::ExpiryPassed { ticker } => write!(
                f,
                "{ticker}: traded after the expiry NSE's last-Thursday rule derives; rule and vendor disagree"
            ),
            Self::ExpiryUnsettled { ticker, why } => {
                write!(f, "{ticker}: expiry unsettled: {why}")
            }
            Self::StrikeImplausible {
                ticker,
                strike,
                median,
            } => write!(
                f,
                "{ticker}: strike implausible: {strike} paisa is more than {STRIKE_PLAUSIBLE_MULTIPLE} times the day's median strike {median} paisa of its underlying; refused, never listed"
            ),
            Self::UnderlyingRefused { ticker } => {
                write!(
                    f,
                    "{ticker}: the underlying is not a symbol the store can file"
                )
            }
            Self::UnderlyingStale { ticker, why } => {
                let (brutex_core::alias::Stale::AfterChange(change)
                | brutex_core::alias::Stale::BeforeChange(change)) = why;
                write!(
                    f,
                    "{ticker}: named for a symbol the exchange was not using that day: {} became {} on {} (NSE symbolchange.csv); refused, never filed under a guessed underlying",
                    change.old, change.new, change.effective
                )
            }
            Self::ContractUnrenderable { ticker } => {
                write!(
                    f,
                    "{ticker}: the contract does not fit the store's contract segment"
                )
            }
            Self::DuplicateTicker { ticker } => {
                write!(f, "{ticker}: two entries of one day name this ticker")
            }
            Self::HeaderUnknown => f.write_str("the first line is not the GDFL header"),
            Self::TickerMismatch { line } => {
                write!(f, "line {line}: the Ticker field is not the file's stem")
            }
            Self::DateMismatch { line } => write!(f, "line {line}: the Date is not the day's"),
            Self::MalformedRow { line } => write!(f, "line {line}: not ten well-formed fields"),
            Self::PriceRefused { line } => write!(
                f,
                "line {line}: the LTP is not a two-place decimal, or a traded row is priced at zero"
            ),
            Self::OpenInterestRefused { line } => {
                write!(f, "line {line}: the open interest does not fit an i64")
            }
            Self::RowsOverCap { cap } => write!(f, "more than {cap} rows"),
            Self::Source(why) => write!(f, "{why}"),
        }
    }
}

impl core::error::Error for NfoRefusal {}

impl From<CmRefusal> for NfoRefusal {
    fn from(why: CmRefusal) -> Self {
        Self::Source(why)
    }
}

/// The first trade day on which every option name is in the day form,
/// 2019-02-01, as days since the epoch (FORMAT.md §6 rule 1; the last trade
/// date with a monthly-only name is 2019-01-31).
pub const DAY_FORM_FROM_DAYS: u32 = 17_928;

/// The first trade day the index weeklies are read in the day form,
/// 2018-09-03, GDFL's first day: the name census has no earlier name, so
/// before it nothing is read as dated (D-2813).
pub const INDEX_WEEKLY_FROM_DAYS: u32 = 17_777;

/// The underlyings whose weeklies use the day form before
/// [`DAY_FORM_FROM_DAYS`] (FORMAT.md §6 rule 2).
pub const EARLY_DAY_FORM: [&str; 2] = ["NIFTY", "BANKNIFTY"];

/// Which vendor form a ticker was read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickerForm {
    /// `UNDERLYING DD MON YY STRIKE SIDE`: the expiry day is stated.
    Day,
    /// `UNDERLYING YY MON STRIKE SIDE`: the expiry day is derived.
    Monthly,
}

/// The contract one option ticker names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionTicker {
    /// The underlying the contract is FILED under (D-3354): the newest
    /// symbol the ticker's own spelling reaches through the dated symbol
    /// changes (`crate::gdfl_lineage::filed_underlying`), which is the
    /// ticker's spelling for every symbol that was never renamed.
    pub underlying: Symbol,
    /// The store's contract segment, keyed on the SETTLED expiry (D-3331).
    pub contract: Contract,
    /// The form the ticker was read in.
    pub form: TickerForm,
    /// The settled expiry: the day the contract expired (D-3331). Equal to
    /// [`Self::stated`] unless a measured re-dating, or the closed-day rule,
    /// settles it elsewhere ([`Self::settlement`] says which).
    pub expiry: Day,
    /// The expiry the vendor's text states (day form) or the rule derives
    /// (monthly form): the alias the settled contract was traded under.
    pub stated: Day,
    /// How [`Self::expiry`] was established.
    pub settlement: Settlement,
    /// The strike, paisa.
    pub strike: i64,
    /// Call or put.
    pub side: OptionSide,
}

impl OptionTicker {
    /// The ticker this reading renders back to, byte for byte the vendor's
    /// when the name decoded (every strike is canonical, so nothing is lost).
    /// Rendered from the STATED expiry: the text is the vendor's.
    #[must_use]
    pub fn ticker_text(&self) -> String {
        let month = MONTHS
            .get(usize::from(self.stated.month()).saturating_sub(1))
            .copied()
            .unwrap_or_default();
        let yy = self.stated.year() % 100;
        let side = match self.side {
            OptionSide::Call => "CE",
            OptionSide::Put => "PE",
        };
        let (rupees, paisa) = (self.strike / 100, self.strike % 100);
        let strike = match paisa {
            0 => rupees.to_string(),
            p if p % 10 == 0 => format!("{rupees}.{}", p / 10),
            p => format!("{rupees}.{p:02}"),
        };
        let under = self.underlying.as_str();
        match self.form {
            TickerForm::Day => format!(
                "{under}{:02}{month}{yy:02}{strike}{side}",
                self.stated.day()
            ),
            TickerForm::Monthly => format!("{under}{yy:02}{month}{strike}{side}"),
        }
    }
}

/// Whether `day` is a weekday. 1970-01-01, day 0, was a Thursday.
const fn is_weekday(day: Day) -> bool {
    (day.days_from_epoch() + 3) % 7 < 5
}

/// The month number a three-letter upper-case month name states.
fn month_of(name: &[u8]) -> Option<u8> {
    match name {
        b"JAN" => Some(1),
        b"FEB" => Some(2),
        b"MAR" => Some(3),
        b"APR" => Some(4),
        b"MAY" => Some(5),
        b"JUN" => Some(6),
        b"JUL" => Some(7),
        b"AUG" => Some(8),
        b"SEP" => Some(9),
        b"OCT" => Some(10),
        b"NOV" => Some(11),
        b"DEC" => Some(12),
        _ => None,
    }
}

/// Two ASCII digits as a number.
fn two_digits(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [a, b] if a.is_ascii_digit() && b.is_ascii_digit() => Some((a - b'0') * 10 + (b - b'0')),
        _ => None,
    }
}

/// A positive strike in paisa, written canonically: digits with no leading
/// zero (a lone `0` before a dot excepted), then optionally a dot and one or
/// two digits, the last not zero. Canonical, so a decoded name renders back
/// to itself and `100` and `100.00` can never be two names of one contract.
fn strike_paisa(text: &[u8]) -> Option<i64> {
    let text = core::str::from_utf8(text).ok()?;
    let (whole, frac) = text
        .split_once('.')
        .map_or((text, None), |(w, f)| (w, Some(f)));
    let whole_ok = !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && (whole == "0" || !whole.starts_with('0'));
    let frac_ok = frac.is_none_or(|f| {
        matches!(f.len(), 1 | 2) && f.bytes().all(|b| b.is_ascii_digit()) && !f.ends_with('0')
    });
    if !(whole_ok && frac_ok) {
        return None;
    }
    crate::csv::paisa(text).filter(|&paisa| paisa > 0)
}

/// The underlying a name's own shape spells, whether or not the name decodes:
/// the text before the two digits and the month its LAST letters spell, the
/// split both the dated `DD MON YY STRIKE` and the monthly `YY MON STRIKE`
/// reading put there. A strike holds no letter, so the month of either
/// reading is the name's last letters, and the split is found from the end
/// whatever follows the month. `None` when the name has no `CE`/`PE` side,
/// its last three letters are no month, no two digits precede them, or
/// nothing precedes those.
///
/// `LTI06APR24100CE` (a Saturday expiry) does not decode, and its shape still
/// says it is `LTI`'s, never `LT`'s: `crate::gdfl_import` asks this before it
/// falls back to the longest underlying a shapeless name starts with
/// (D-2818). Neither does `LTI06APR24100.125CE` nor a name past the length
/// cap, and they are `LTI`'s too: a reading that asked for a strike that
/// parsed and a name inside the cap handed such names back to `LT` (D-2823).
/// Every name the strict readings shape, this shapes at the same split.
/// O(ticker length), with no cap.
#[must_use]
pub(crate) fn shaped_underlying(ticker: &str) -> Option<&str> {
    let body = ticker
        .strip_suffix("CE")
        .or_else(|| ticker.strip_suffix("PE"))?;
    let bytes = body.as_bytes();
    let mut last = None;
    for (at, byte) in bytes.iter().enumerate() {
        if byte.is_ascii_alphabetic() {
            last = Some(at);
        }
    }
    let at = last?.checked_sub(4).filter(|&at| at > 0)?;
    two_digits(bytes.get(at..at + 2)?)?;
    month_of(bytes.get(at + 2..at + 5)?)?;
    body.get(..at)
}

/// The expiry day of the monthly contract of `month` of `year`: the last
/// Thursday, or the trading day before it when it is not one (NSE's rule,
/// charter GDFL options section), walked on the calendar. `None` when the
/// calendar has not measured a day the walk needs.
fn monthly_expiry(year: u16, month: u8) -> Option<Day> {
    let end = Day::new(year, month, 1).ok()?.end_of_month();
    // Back from the month's end to its last Thursday: day 0 was a Thursday.
    let mut day = end.days_from_epoch() - end.days_from_epoch() % 7;
    // Ends: every walk back reaches an open day or the calendar's first day.
    loop {
        match crate::calendar::sourced_kind_of(i64::from(day)) {
            crate::calendar::DayKind::Open(_) => return Day::from_days(day).ok(),
            crate::calendar::DayKind::Unmeasured => return None,
            _ => day -= 1,
        }
    }
}

/// One reading of a ticker before its underlying is checked.
struct Reading {
    at: usize,
    form: TickerForm,
    expiry: Day,
    strike: i64,
}

/// Every reading the era admits for the split at `at` (FORMAT.md §6), the
/// refusals a monthly reading meets kept as readings that refuse.
fn readings_at(
    body: &[u8],
    at: usize,
    trade: Day,
    out: &mut Vec<Result<Reading, NfoRefusal>>,
    ticker: &str,
) {
    let (Some(under), Some(rest)) = (body.get(..at), body.get(at..)) else {
        return;
    };
    let latest = trade.days_from_epoch().saturating_add(EXPIRY_HORIZON_DAYS);
    let modern = trade.days_from_epoch() >= DAY_FORM_FROM_DAYS;
    let index_weekly = trade.days_from_epoch() >= INDEX_WEEKLY_FROM_DAYS
        && EARLY_DAY_FORM.iter().any(|name| name.as_bytes() == under);
    let mut had_day = false;
    if modern || index_weekly {
        let day_form = (|| {
            let dd = rest.get(0..2).and_then(two_digits)?;
            let mon = rest.get(2..5).and_then(month_of)?;
            let yy = rest.get(5..7).and_then(two_digits)?;
            let strike = rest.get(7..).and_then(strike_paisa)?;
            let expiry = Day::new(2000 + u16::from(yy), mon, dd).ok()?;
            // Before the switch a dated index name is a contract of the three-
            // month cycle ("3 month trading cycle - the near month, the next
            // month and the far month", NSE contract specifications, archived
            // 2019-08-16; D-2813): its expiry month is at most three months on.
            let months_on = (u32::from(expiry.year()) * 12 + u32::from(expiry.month()))
                .saturating_sub(u32::from(trade.year()) * 12 + u32::from(trade.month()));
            (is_weekday(expiry)
                && expiry.days_from_epoch() >= trade.days_from_epoch()
                && expiry.days_from_epoch() <= latest
                && (modern || months_on <= 3))
                .then_some(Reading {
                    at,
                    form: TickerForm::Day,
                    expiry,
                    strike,
                })
        })();
        had_day = day_form.is_some();
        out.extend(day_form.map(Ok));
    }
    // AN INDEX WEEKLY'S DAY READING WINS over its monthly reading at the same
    // split (`BANKNIFTY19SEP1824800PE`: Wed 19 Sep 2018, the holiday-shifted
    // weekly, never the Sep 2019 monthly strike 18,24,800). Measured over
    // every one of the 1,218,362 tickers: this is the tick store's reading
    // for all 2,240 pre-switch index weeklies, each last traded on or before
    // that day; an index MONTHLY has no day reading and stays monthly.
    if !(modern || (index_weekly && had_day)) {
        let monthly = (|| {
            let yy = rest.get(0..2).and_then(two_digits)?;
            let mon = rest.get(2..5).and_then(month_of)?;
            let strike = rest.get(5..).and_then(strike_paisa)?;
            let year = 2000 + u16::from(yy);
            let first = Day::new(year, mon, 1).ok()?;
            let in_window =
                (year, mon) >= (trade.year(), trade.month()) && first.days_from_epoch() <= latest;
            in_window.then_some((year, mon, strike))
        })();
        if let Some((year, mon, strike)) = monthly {
            out.push(match monthly_expiry(year, mon) {
                None => Err(NfoRefusal::MonthlyExpiryUnmeasured {
                    ticker: ticker.to_owned(),
                }),
                Some(expiry) if expiry.days_from_epoch() < trade.days_from_epoch() => {
                    Err(NfoRefusal::ExpiryPassed {
                        ticker: ticker.to_owned(),
                    })
                }
                Some(expiry) => Ok(Reading {
                    at,
                    form: TickerForm::Monthly,
                    expiry,
                    strike,
                }),
            });
        }
    }
}

/// The one reading of `ticker`, or its refusal: none is `TickerUnparsed`, two
/// or more `TickerAmbiguous` (FORMAT.md §6 rule 4). Over every one of the
/// 1,218,362 tickers of the vendor's history no name has two readings under
/// the era rule; the guard stays, because a name the vendor ships tomorrow is
/// not in that census.
fn single(
    mut readings: Vec<Result<Reading, NfoRefusal>>,
    ticker: &str,
) -> Result<Reading, NfoRefusal> {
    if readings.len() > 1 {
        return Err(NfoRefusal::TickerAmbiguous {
            ticker: ticker.to_owned(),
        });
    }
    readings.pop().unwrap_or_else(|| {
        Err(NfoRefusal::TickerUnparsed {
            ticker: ticker.to_owned(),
        })
    })
}

/// The contract `ticker`, a file stem without `.NFO`, names for a file of
/// trade day `trade`, by FORMAT.md §6 (module doc).
///
/// # Errors
///
/// [`NfoRefusal::TickerUnparsed`] (no reading), [`NfoRefusal::TickerAmbiguous`]
/// (more than one), [`NfoRefusal::MonthlyExpiryUnmeasured`],
/// [`NfoRefusal::ExpiryPassed`], [`NfoRefusal::UnderlyingRefused`],
/// [`NfoRefusal::UnderlyingStale`], [`NfoRefusal::ExpiryUnsettled`] and [`NfoRefusal::ContractUnrenderable`].
/// The strike's plausibility needs the day's ladder and is checked by
/// [`read_file`] ([`NfoDay::check_strike`]), not here.
pub fn decode_ticker(ticker: &str, trade: Day) -> Result<OptionTicker, NfoRefusal> {
    let unparsed = || NfoRefusal::TickerUnparsed {
        ticker: ticker.to_owned(),
    };
    if ticker.len() > TICKER_CAP {
        return Err(unparsed());
    }
    let (body, side) = match ticker.as_bytes().split_last_chunk::<2>() {
        Some((body, b"CE")) => (body, OptionSide::Call),
        Some((body, b"PE")) => (body, OptionSide::Put),
        _ => return Err(unparsed()),
    };
    let mut readings = Vec::new();
    for at in 1..body.len() {
        readings_at(body, at, trade, &mut readings, ticker);
    }
    let reading = single(readings, ticker)?;
    let name = ticker.get(..reading.at).unwrap_or_default();
    let underlying = Symbol::new(name)
        .ok()
        .filter(|symbol| symbol.as_str() == name)
        .ok_or_else(|| NfoRefusal::UnderlyingRefused {
            ticker: ticker.to_owned(),
        })?;
    // ONE CONTRACT ACROSS A SYMBOL CHANGE (DB-B05, DB-B11, DB-B62; D-3333):
    // the file is filed under the newest symbol the name's dated changes
    // reach, as the cash side files an old stem, so a contract alive across
    // `NIITTECH -> COFORGE` is one series. The vendor's text stays the alias
    // (`brutex_core::alias::LINEAGE.backward`).
    let underlying = crate::gdfl_lineage::filed_underlying(underlying, trade).map_err(|why| {
        NfoRefusal::UnderlyingStale {
            ticker: ticker.to_owned(),
            why,
        }
    })?;
    // THE SETTLED EXPIRY IS THE IDENTITY (D-3331). A re-dated contract is one
    // series under its final day; a closed stated day no row names settles on
    // the previous open session the calendar records (the charter's rule),
    // and one with no such session is refused, never kept as stated.
    let settled = settle(underlying.as_str(), reading.expiry, Some(trade)).map_err(|why| {
        NfoRefusal::ExpiryUnsettled {
            ticker: ticker.to_owned(),
            why,
        }
    })?;
    let contract = Expiry::new(
        settled.expiry.year(),
        settled.expiry.month(),
        settled.expiry.day(),
    )
    .ok()
    .and_then(|expiry| {
        Contract::of(Kind::Option {
            expiry,
            strike: Paisa::from_raw(reading.strike),
            side,
        })
    })
    .ok_or_else(|| NfoRefusal::ContractUnrenderable {
        ticker: ticker.to_owned(),
    })?;
    Ok(OptionTicker {
        underlying,
        contract,
        form: reading.form,
        expiry: settled.expiry,
        stated: reading.expiry,
        settlement: settled.settlement,
        strike: reading.strike,
        side,
    })
}

/// `GFDLNFO_TICK_03082026`: the day folder's name.
#[must_use]
pub fn day_folder_name(day: Day) -> String {
    format!(
        "{FOLDER_PREFIX}{:02}{:02}{:04}",
        day.day(),
        day.month(),
        day.year()
    )
}

/// `AUG_2026`: the month folder the vendor files a day under.
fn month_folder(day: Day) -> String {
    let month = MONTHS
        .get(usize::from(day.month()).saturating_sub(1))
        .copied()
        .unwrap_or_default();
    format!("{month}_{:04}", day.year())
}

/// The ticker an entry name files, when it is an option file of `folder`:
/// `<folder>\Options\<TICKER>.NFO.csv` (either separator).
fn entry_ticker<'a>(folder: &str, entry: &'a str) -> Option<&'a str> {
    let rest = entry.strip_prefix(folder)?;
    let rest = rest.strip_prefix('\\').or_else(|| rest.strip_prefix('/'))?;
    let rest = rest.strip_prefix(OPTIONS_FOLDER)?;
    let file = rest.strip_prefix('\\').or_else(|| rest.strip_prefix('/'))?;
    NFO_SUFFIXES
        .iter()
        .find_map(|suffix| file.strip_suffix(suffix))
        .filter(|ticker| !ticker.is_empty() && !ticker.contains(['\\', '/']))
}

/// One options day as a source lists it: every entry in the source's order,
/// each option file also filed by its ticker, so "does contract X have a
/// file this day" is one hash probe.
#[derive(Debug, Clone)]
pub struct NfoDay<L> {
    day: Day,
    folder: String,
    entries: Vec<ListedFile<L>>,
    by_ticker: HashMap<Box<str>, usize>,
    duplicates: HashSet<Box<str>>,
    skipped: usize,
    /// Each underlying's median strike over the day's decodable names,
    /// computed on the first ask and dropped by every [`Self::push`].
    medians: OnceLock<HashMap<Box<str>, i64>>,
    /// The day zip's own size, where the source read one (D-3335's
    /// `member_bytes`); `None` for the tick store, which holds no zip.
    member_bytes: Option<u64>,
}

impl<L> NfoDay<L> {
    /// An empty listing of `day`, sized for the `entries` its source states.
    #[must_use]
    pub fn new(day: Day, entries: usize) -> Self {
        Self {
            day,
            folder: day_folder_name(day),
            entries: Vec::with_capacity(entries),
            by_ticker: HashMap::with_capacity(entries),
            duplicates: HashSet::with_capacity(1),
            skipped: 0,
            medians: OnceLock::new(),
            member_bytes: None,
        }
    }

    /// This listing, read from a day zip of `bytes` stored bytes (D-3335's
    /// `member_bytes`).
    #[must_use]
    pub const fn with_member_bytes(mut self, bytes: u64) -> Self {
        self.member_bytes = Some(bytes);
        self
    }

    /// The day zip's size, when the source read one: `None` for the tick
    /// store, which keeps each member and not the zip.
    #[must_use]
    pub const fn member_bytes(&self) -> Option<u64> {
        self.member_bytes
    }

    /// Appends one entry in source order. An option file of the day folder is
    /// filed by its ticker; anything else is kept and counted, never filed.
    pub fn push(&mut self, entry: &str, len: u64, crc32: u32, locator: L) {
        // A median computed before this entry would not be the day's.
        self.medians = OnceLock::new();
        let at = self.entries.len();
        match entry_ticker(&self.folder, entry) {
            Some(ticker) => {
                if self.by_ticker.insert(ticker.into(), at).is_some() {
                    self.duplicates.insert(ticker.into());
                }
            }
            None => self.skipped += 1,
        }
        self.entries.push(ListedFile {
            entry: entry.into(),
            len,
            crc32,
            locator,
        });
    }

    /// The day listed.
    #[must_use]
    pub const fn day(&self) -> Day {
        self.day
    }

    /// Every entry, in the source's order.
    #[must_use]
    pub fn entries(&self) -> &[ListedFile<L>] {
        &self.entries
    }

    /// How many entries are not option files of the day folder.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// The ticker of `file` when it is an option file of this day.
    #[must_use]
    pub fn ticker_of<'a>(&self, file: &'a ListedFile<L>) -> Option<&'a str> {
        entry_ticker(&self.folder, &file.entry)
    }

    /// The file holding `ticker`, if the day has one: one hash probe.
    ///
    /// # Errors
    ///
    /// [`NfoRefusal::DuplicateTicker`] when two entries name it.
    pub fn locate(&self, ticker: &str) -> Result<Option<&ListedFile<L>>, NfoRefusal> {
        if self.duplicates.contains(ticker) {
            return Err(NfoRefusal::DuplicateTicker {
                ticker: ticker.to_owned(),
            });
        }
        Ok(self
            .by_ticker
            .get(ticker)
            .and_then(|&at| self.entries.get(at)))
    }

    /// The LOWER median strike, paisa, of `underlying`'s decodable option
    /// names this day (every expiry and side; a name that does not decode is
    /// not in it), or `None` when the day lists none. The day's own ladder,
    /// whatever a run's filter asks for, so the answer does not depend on the
    /// filter. Computed for every underlying on the first ask
    /// (O(entries · log entries), once per listing); after it, one hash
    /// probe.
    #[must_use]
    pub fn strike_median(&self, underlying: &str) -> Option<i64> {
        self.medians
            .get_or_init(|| {
                let mut strikes: HashMap<Box<str>, Vec<i64>> = HashMap::new();
                for file in &self.entries {
                    let decoded = entry_ticker(&self.folder, &file.entry)
                        .map(|ticker| decode_ticker(ticker, self.day));
                    if let Some(Ok(ticker)) = decoded {
                        strikes
                            .entry(ticker.underlying.as_str().into())
                            .or_default()
                            .push(ticker.strike);
                    }
                }
                strikes
                    .into_iter()
                    .filter_map(|(underlying, mut all)| {
                        lower_median(&mut all).map(|median| (underlying, median))
                    })
                    .collect()
            })
            .get(underlying)
            .copied()
    }

    /// Whether `ticker`'s strike is one the day's ladder admits: at most
    /// [`STRIKE_PLAUSIBLE_MULTIPLE`] times its underlying's
    /// [`Self::strike_median`] (D-3331).
    ///
    /// # Errors
    ///
    /// [`NfoRefusal::StrikeImplausible`], naming the ticker, the strike and
    /// the median.
    pub fn check_strike(&self, ticker: &OptionTicker, name: &str) -> Result<(), NfoRefusal> {
        match self.strike_median(ticker.underlying.as_str()) {
            Some(median) if strike_implausible(ticker.strike, median) => {
                Err(NfoRefusal::StrikeImplausible {
                    ticker: name.to_owned(),
                    strike: ticker.strike,
                    median,
                })
            }
            _ => Ok(()),
        }
    }
}

/// Where the vendor's options files are read from (D-2806): the yearly zips
/// ([`NfoZips`]) or the verified tick store ([`NfoTickStore`]). One per run,
/// never both, with no fallback between them.
pub trait NfoSource {
    /// Where the source keeps one file's bytes.
    type Locator;

    /// The listing of `day`, or `None` when the source holds no such day.
    ///
    /// # Errors
    ///
    /// The source's own refusals.
    fn day(&self, day: Day) -> Result<Option<NfoDay<Self::Locator>>, CmRefusal>;

    /// The bytes of `file`, unverified: [`read_file`] checks them.
    ///
    /// # Errors
    ///
    /// The source's own refusals.
    fn fetch(&self, file: &ListedFile<Self::Locator>) -> Result<Vec<u8>, CmRefusal>;
}

/// The tick store's options tree (FORMAT.md §2):
/// `options/<yyyy>/<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.bts`. Only that exact
/// name is opened, so a `*.bts.tmp` is never read. Read-only, positional
/// reads, nothing mapped.
#[derive(Debug, Clone)]
pub struct NfoTickStore {
    root: PathBuf,
}

impl NfoTickStore {
    /// The store whose root is `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Where the day file of `day` lies.
    #[must_use]
    pub fn day_path(&self, day: Day) -> PathBuf {
        self.root.join(format!(
            "options/{:04}/{}/{}.bts",
            day.year(),
            month_folder(day),
            day_folder_name(day)
        ))
    }
}

/// Where one entry's block lies in a tick-store day file.
#[derive(Debug, Clone)]
pub struct StoreLocator {
    file: Arc<std::fs::File>,
    entry: IndexEntry,
}

impl NfoSource for NfoTickStore {
    type Locator = StoreLocator;

    fn day(&self, day: Day) -> Result<Option<NfoDay<StoreLocator>>, CmRefusal> {
        // THE STORE'S ONE READING DOOR (R3-16, FM-046): a link, a pipe or a
        // device at a day's name is refused by name, never read as absent.
        let Some((file, len)) = crate::gdfl_tickstore::open_day(&self.day_path(day))? else {
            return Ok(None);
        };
        let entries = read_index(&file, len)?;
        let file = Arc::new(file);
        let mut listing = NfoDay::new(day, entries.len());
        for entry in entries {
            let (name, size, crc) = (entry.name.clone(), entry.size, entry.crc);
            listing.push(
                &name,
                size,
                crc,
                StoreLocator {
                    file: Arc::clone(&file),
                    entry,
                },
            );
        }
        Ok(Some(listing))
    }

    fn fetch(&self, file: &ListedFile<StoreLocator>) -> Result<Vec<u8>, CmRefusal> {
        rebuild(&*file.locator.file, &file.locator.entry)
    }
}

/// The vendor's yearly options zips in one directory, `<yyyy>.zip` each,
/// read in place: the year zip opened and its central directory walked ONCE
/// per value and year (O(entries in the year zip), 84 to 265 measured), then
/// each day one hash probe for its day zip, the day zip's own directory once
/// (O(its entries)), and a file by one local header and one inflate. Nothing
/// is extracted.
///
/// # The year's directory once per run (WP-16, design F6)
///
/// [`NfoSource::day`] reopened the year zip and walked its whole central
/// directory for every day it was asked, about 260 records to find one name,
/// 250 times a year. A run holds one `NfoZips`, so the year zip is opened and
/// its directory indexed the first time a day of that year is asked, and the
/// open file is reused by every later day and every file read of the run
/// (`gdfl_nfo_tests::a_year_zip_is_indexed_once_and_each_day_is_one_lookup`).
/// A year whose zip is absent is asked again each day, so a zip that appears
/// mid-run is read; one that refuses is not kept, so each day names the
/// refusal as before. Clones share the index.
#[derive(Debug, Clone)]
pub struct NfoZips {
    dir: PathBuf,
    years: Arc<std::sync::Mutex<HashMap<u16, Arc<YearIndex>>>>,
}

/// One year zip, open, and its day zips by exact entry name: the entry, or
/// [`None`] where two records share the name, which every day of that name
/// refuses as [`CmRefusal::ArchiveDuplicateDay`].
#[derive(Debug)]
struct YearIndex {
    file: Arc<std::fs::File>,
    days: HashMap<String, Option<ZipEntry>>,
}

impl NfoZips {
    /// The zips in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            years: Arc::new(std::sync::Mutex::new(HashMap::with_capacity(16))),
        }
    }

    /// The index of `year`'s zip, built on first use and kept; [`None`] when
    /// the zip is absent.
    ///
    /// # Errors
    ///
    /// The zip refusals of [`zip_entries`], and
    /// [`CmRefusal::ArchiveUnavailable`] for a zip that cannot be opened or
    /// measured.
    fn year(&self, year: u16) -> Result<Option<Arc<YearIndex>>, CmRefusal> {
        let unavailable = |why: std::io::Error| CmRefusal::ArchiveUnavailable { kind: why.kind() };
        // A poisoned index is still a correct one: every value in it was
        // inserted whole, after its walk finished.
        let mut years = self
            .years
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = years.get(&year) {
            return Ok(Some(Arc::clone(index)));
        }
        let path = self.dir.join(format!("{year:04}.zip"));
        let file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(unavailable(why)),
        };
        let len = file.metadata().map_err(unavailable)?.len();
        let entries = zip_entries(&file, 0, len)?;
        let mut days: HashMap<String, Option<ZipEntry>> = HashMap::with_capacity(entries.len());
        for entry in entries {
            match days.entry(entry.name.clone()) {
                std::collections::hash_map::Entry::Occupied(mut twice) => {
                    twice.insert(None);
                }
                std::collections::hash_map::Entry::Vacant(first) => {
                    first.insert(Some(entry));
                }
            }
        }
        let index = Arc::new(YearIndex {
            file: Arc::new(file),
            days,
        });
        years.insert(year, Arc::clone(&index));
        Ok(Some(index))
    }
}

/// Where one member of an options day zip lies: the year zip, shared by
/// every entry of the day, and the member inside it.
#[derive(Debug)]
pub struct ZipFileLocator<B = std::fs::File> {
    file: Arc<B>,
    at: ZipLocator,
}

impl NfoSource for NfoZips {
    type Locator = ZipFileLocator;

    fn day(&self, day: Day) -> Result<Option<NfoDay<ZipFileLocator>>, CmRefusal> {
        let Some(year) = self.year(day.year())? else {
            return Ok(None);
        };
        let want = format!("{}/{}.zip", month_folder(day), day_folder_name(day));
        match year.days.get(&want) {
            None => Ok(None),
            Some(None) => Err(CmRefusal::ArchiveDuplicateDay { day }),
            Some(Some(inner)) => day_listing(&year.file, inner, day).map(Some),
        }
    }

    fn fetch(&self, file: &ListedFile<ZipFileLocator>) -> Result<Vec<u8>, CmRefusal> {
        member_bytes(&*file.locator.file, file.locator.at, &file.entry, file.len)
    }
}

/// The listing of `day` from the year zip `file` of `len` bytes: its central
/// directory walked once for the day zip's exact name, a duplicate refused,
/// then the stored day zip's own directory read in place.
///
/// # Errors
///
/// The zip refusals of [`zip_entries`] and [`stored_span`], and
/// [`CmRefusal::ArchiveDuplicateDay`] for two day zips of one day.
pub fn listing_in<B: ReadAt>(
    file: &Arc<B>,
    len: u64,
    day: Day,
) -> Result<Option<NfoDay<ZipFileLocator<B>>>, CmRefusal> {
    let want = format!("{}/{}.zip", month_folder(day), day_folder_name(day));
    let mut inner: Option<ZipEntry> = None;
    for entry in zip_entries(&**file, 0, len)? {
        if entry.name == want {
            if inner.is_some() {
                return Err(CmRefusal::ArchiveDuplicateDay { day });
            }
            inner = Some(entry);
        }
    }
    let Some(inner) = inner else {
        return Ok(None);
    };
    day_listing(file, &inner, day).map(Some)
}

/// The listing of `day` from its stored day zip `inner`, a record of the year
/// zip `file`: the day zip's own directory, read in place.
///
/// # Errors
///
/// The zip refusals of [`zip_entries`] and [`stored_span`].
fn day_listing<B: ReadAt>(
    file: &Arc<B>,
    inner: &ZipEntry,
    day: Day,
) -> Result<NfoDay<ZipFileLocator<B>>, CmRefusal> {
    let (base, span) = stored_span(&**file, inner)?;
    let members = zip_entries(&**file, base, span)?;
    let mut listing = NfoDay::new(day, members.len()).with_member_bytes(span);
    for entry in members {
        listing.push(
            &entry.name,
            entry.len,
            entry.crc32,
            ZipFileLocator {
                file: Arc::clone(file),
                at: entry.locator,
            },
        );
    }
    Ok(listing)
}

/// One decoded row, in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NfoRow {
    /// One-based line number.
    pub line: u32,
    /// Second of the IST day the row is stamped at.
    pub sod: u32,
    /// Last traded price in paisa, exactly as written to two places. May be
    /// zero only on a row with no traded quantity.
    pub ltp: i64,
    /// Last traded quantity.
    pub ltq: u64,
    /// Open interest.
    pub oi: i64,
}

/// A decoded options file: every row, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfoFile {
    /// Every row, none dropped, none moved.
    pub rows: Vec<NfoRow>,
    /// Rows whose bid or ask is not zero. A file with rows and none of these
    /// is D-3335's `zero_quote_files` (30 May 2019: 10,435 of 10,453 traded
    /// files); counted, never a refusal.
    pub quoted_rows: u32,
}

/// Whether `text`, a plain decimal already checked, is zero: every digit `0`.
fn is_zero_decimal(text: &str) -> bool {
    text.bytes().all(|b| b == b'0' || b == b'.')
}

/// `HH:MM:SS` as a second of the day.
fn second_of_day(time: &str) -> Option<u32> {
    let &[h1, h2, b':', m1, m2, b':', s1, s2] = time.as_bytes() else {
        return None;
    };
    let hours = u32::from(two_digits(&[h1, h2])?);
    let minutes = u32::from(two_digits(&[m1, m2])?);
    let seconds = u32::from(two_digits(&[s1, s2])?);
    (hours <= 23 && minutes <= 59 && seconds <= 59)
        .then_some(hours * 3_600 + minutes * 60 + seconds)
}

/// Digits, optionally a dot and more digits.
fn is_decimal(text: &str) -> bool {
    match text.split_once('.') {
        Some((whole, frac)) => is_digits(whole) && is_digits(frac),
        None => is_digits(text),
    }
}

/// One or more ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// Decodes one options file whose stem is `stem` (`<TICKER>.NFO`) for `day`.
/// Crate-private: outside this crate the only way from a source's bytes to
/// rows is [`read_file`], which checks length and CRC-32 first.
///
/// # Errors
///
/// Every row refusal of [`NfoRefusal`], naming the first line that failed.
pub(crate) fn decode(bytes: &[u8], stem: &str, day: Day) -> Result<NfoFile, NfoRefusal> {
    decode_capped(bytes, stem, day, crate::fetch::MAX_ROWS)
}

/// [`decode`] with the row bound as a parameter.
fn decode_capped(bytes: &[u8], stem: &str, day: Day, cap: usize) -> Result<NfoFile, NfoRefusal> {
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let mut lines = body.split(|&b| b == b'\n');
    match lines
        .next()
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
    {
        Some(h)
            if h == HEADER_OPEN_INTEREST.as_bytes()
                || h == HEADER_OPEN_INTEREST_SPACED.as_bytes() => {}
        _ => return Err(NfoRefusal::HeaderUnknown),
    }
    let date = format!("{:02}/{:02}/{:04}", day.day(), day.month(), day.year());
    // RESERVED ONCE, FROM THE BODY, as `csv::decode_rows` is (D-1203): every
    // row is one line after the header, so the body's newlines bound the
    // rows, and `cap` bounds the reservation as it bounds the rows kept.
    let bound = body.split(|&byte| byte == b'\n').count().min(cap);
    let mut rows = Vec::with_capacity(bound);
    let mut quoted_rows = 0_u32;
    for (index, raw) in lines.enumerate() {
        let line = u32::try_from(index.saturating_add(2)).unwrap_or(u32::MAX);
        if rows.len() >= cap {
            return Err(NfoRefusal::RowsOverCap { cap });
        }
        let malformed = NfoRefusal::MalformedRow { line };
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let text = core::str::from_utf8(raw).map_err(|_| malformed.clone())?;
        // TEN FIELDS INTO A FIXED ARRAY, NOT A VECTOR PER ROW: the
        // capital-market reader's `parse_row` shape. An eleventh field is a
        // malformed row the moment it is seen, so a line of a million commas
        // costs eleven steps, not a million-slot allocation.
        let mut fields = [""; 10];
        let mut count = 0_usize;
        for field in text.split(',') {
            let Some(slot) = fields.get_mut(count) else {
                return Err(malformed);
            };
            *slot = field;
            count = count.saturating_add(1);
        }
        if count != fields.len() {
            return Err(malformed);
        }
        let [
            ticker,
            row_date,
            time,
            ltp,
            bid,
            bid_qty,
            ask,
            ask_qty,
            ltq,
            oi,
        ] = fields;
        if ticker != stem {
            return Err(NfoRefusal::TickerMismatch { line });
        }
        if row_date != date {
            return Err(NfoRefusal::DateMismatch { line });
        }
        let sod = second_of_day(time).ok_or_else(|| malformed.clone())?;
        if !(is_decimal(bid) && is_decimal(ask) && is_digits(bid_qty) && is_digits(ask_qty)) {
            return Err(malformed);
        }
        if !(is_zero_decimal(bid) && is_zero_decimal(ask)) {
            quoted_rows = quoted_rows.saturating_add(1);
        }
        let ltq: u64 = if is_digits(ltq) {
            ltq.parse().ok()
        } else {
            None
        }
        .ok_or_else(|| malformed.clone())?;
        if !is_digits(oi) {
            return Err(malformed);
        }
        let oi: i64 = oi
            .parse()
            .map_err(|_| NfoRefusal::OpenInterestRefused { line })?;
        let paisa = if is_decimal(ltp) {
            crate::csv::paisa(ltp)
        } else {
            None
        }
        .ok_or(NfoRefusal::PriceRefused { line })?;
        // A TRADED ROW AT ZERO IS REFUSED; ONE PAISA IS A PRICE. Options of
        // low-priced shares print at 0.01 to 0.04 (measured on 2026-08-03: 20
        // files, IDEA, YESBANK, IRFC and others), so the capital-market
        // reader's five-paisa floor is not this tree's (D-2806).
        if ltq > 0 && paisa < 1 {
            return Err(NfoRefusal::PriceRefused { line });
        }
        rows.push(NfoRow {
            line,
            sod,
            ltp: paisa,
            ltq,
            oi,
        });
    }
    Ok(NfoFile { rows, quoted_rows })
}

/// One option file of a day, located, fetched, checked and decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfoDayFile {
    /// The contract the ticker names.
    pub ticker: OptionTicker,
    /// The ticker as the vendor spelled it.
    pub name: String,
    /// Every row.
    pub file: NfoFile,
}

/// Reads `file`, one of `listing`'s option entries: the ticker decoded, the
/// bytes fetched and checked against the length and CRC-32 the listing
/// states BEFORE anything parses them, then decoded.
///
/// # Errors
///
/// The ticker refusals of [`decode_ticker`], [`NfoRefusal::StrikeImplausible`]
/// against the day's ladder ([`NfoDay::check_strike`], before any byte is
/// read), the source's own, the capital-market reader's
/// `SourceLengthMismatch` / `SourceCrcMismatch`, and every refusal of
/// [`decode`]. A file that is not an option file of the day is
/// [`NfoRefusal::TickerUnparsed`].
pub fn read_file<S: NfoSource>(
    source: &S,
    listing: &NfoDay<S::Locator>,
    file: &ListedFile<S::Locator>,
) -> Result<NfoDayFile, NfoRefusal> {
    let name = listing
        .ticker_of(file)
        .ok_or_else(|| NfoRefusal::TickerUnparsed {
            ticker: file.entry.to_string(),
        })?;
    // A duplicate name is refused before anything is read.
    listing.locate(name)?;
    let ticker = decode_ticker(name, listing.day())?;
    listing.check_strike(&ticker, name)?;
    let bytes = source.fetch(file)?;
    crate::gdfl_cm::verify(file, &bytes)?;
    let decoded = decode(&bytes, &format!("{name}{ROW_SUFFIX}"), listing.day())?;
    Ok(NfoDayFile {
        ticker,
        name: name.to_owned(),
        file: decoded,
    })
}

#[cfg(test)]
#[path = "gdfl_nfo_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gdfl_nfo_attack_tests.rs"]
mod attack_tests;
