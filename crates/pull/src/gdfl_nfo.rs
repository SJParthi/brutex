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
//! options section), walked back on [`crate::calendar::kind_of`]. A day the
//! calendar has not measured refuses (`MonthlyExpiryUnmeasured`), and a file
//! traded AFTER its derived expiry refuses (`ExpiryPassed`): that is the
//! per-day half of the cross-check against the vendor's own appearances.
//!
//! The strike must be written canonically (no leading zero, no trailing
//! fractional zero, at most two places), so every name that decodes renders
//! back to itself byte for byte ([`OptionTicker::ticker_text`]). The
//! underlying may hold any byte a `Symbol` admits, digits included (`M&M`,
//! `NAM-INDIA`, `360ONE`, `TV18BRDCST`). An expiry the day form states on an
//! exchange holiday is kept as stated: the name is the contract's identity.
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
//! A day listing is O(entries) once per day; after it [`NfoDay::locate`] is
//! one hash probe (expected O(1), measured by `C-GI-01` in
//! `crates/pull/benches/ratio.rs`). [`decode_ticker`] is O(ticker length),
//! bounded by the 64-byte cap it refuses past. [`decode`] is O(bytes).
//! Fetching a file is O(its compressed and rebuilt bytes). The listing and
//! the fetch are limits in `docs/06-limits.md`, not constant-time claims.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;

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
    /// The underlying is not a symbol the store can file.
    UnderlyingRefused {
        /// The ticker.
        ticker: String,
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
            Self::UnderlyingRefused { ticker } => {
                write!(
                    f,
                    "{ticker}: the underlying is not a symbol the store can file"
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
    /// The underlying's symbol, exactly as the ticker spells it.
    pub underlying: Symbol,
    /// The store's contract segment.
    pub contract: Contract,
    /// The form the ticker was read in.
    pub form: TickerForm,
    /// The expiry: stated (day form) or derived (monthly form).
    pub expiry: Day,
    /// The strike, paisa.
    pub strike: i64,
    /// Call or put.
    pub side: OptionSide,
}

impl OptionTicker {
    /// The ticker this reading renders back to, byte for byte the vendor's
    /// when the name decoded (every strike is canonical, so nothing is lost).
    #[must_use]
    pub fn ticker_text(&self) -> String {
        let month = MONTHS
            .get(usize::from(self.expiry.month()).saturating_sub(1))
            .copied()
            .unwrap_or_default();
        let yy = self.expiry.year() % 100;
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
                self.expiry.day()
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

/// The expiry day of the monthly contract of `month` of `year`: the last
/// Thursday, or the trading day before it when it is not one (NSE's rule,
/// charter GDFL options section), walked on the calendar. `None` when the
/// calendar has not measured a day the walk needs.
fn monthly_expiry(year: u16, month: u8) -> Option<Day> {
    let end = Day::new(year, month, 1).ok()?.end_of_month();
    // Back from the month's end to its last Thursday: day 0 was a Thursday.
    let mut day = end.days_from_epoch() - end.days_from_epoch() % 7;
    for _ in 0..31 {
        match crate::calendar::kind_of(i64::from(day)) {
            crate::calendar::DayKind::Open(_) => return Day::from_days(day).ok(),
            crate::calendar::DayKind::Unmeasured => return None,
            _ => day -= 1,
        }
    }
    None
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
    let index_weekly = EARLY_DAY_FORM.iter().any(|name| name.as_bytes() == under);
    let mut had_day = false;
    if modern || index_weekly {
        let day_form = (|| {
            let dd = rest.get(0..2).and_then(two_digits)?;
            let mon = rest.get(2..5).and_then(month_of)?;
            let yy = rest.get(5..7).and_then(two_digits)?;
            let strike = rest.get(7..).and_then(strike_paisa)?;
            let expiry = Day::new(2000 + u16::from(yy), mon, dd).ok()?;
            (is_weekday(expiry)
                && expiry.days_from_epoch() >= trade.days_from_epoch()
                && expiry.days_from_epoch() <= latest)
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
/// [`NfoRefusal::ExpiryPassed`], [`NfoRefusal::UnderlyingRefused`] and
/// [`NfoRefusal::ContractUnrenderable`].
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
    let contract = Expiry::new(
        reading.expiry.year(),
        reading.expiry.month(),
        reading.expiry.day(),
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
        expiry: reading.expiry,
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
    duplicates: Vec<String>,
    skipped: usize,
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
            duplicates: Vec::new(),
            skipped: 0,
        }
    }

    /// Appends one entry in source order. An option file of the day folder is
    /// filed by its ticker; anything else is kept and counted, never filed.
    pub fn push(&mut self, entry: &str, len: u64, crc32: u32, locator: L) {
        let at = self.entries.len();
        match entry_ticker(&self.folder, entry) {
            Some(ticker) => {
                if self.by_ticker.insert(ticker.into(), at).is_some() {
                    self.duplicates.push(ticker.to_owned());
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
        if self.duplicates.iter().any(|held| held == ticker) {
            return Err(NfoRefusal::DuplicateTicker {
                ticker: ticker.to_owned(),
            });
        }
        Ok(self
            .by_ticker
            .get(ticker)
            .and_then(|&at| self.entries.get(at)))
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
        let unavailable =
            |why: std::io::Error| CmRefusal::TickStoreUnavailable { kind: why.kind() };
        let file = match std::fs::File::open(self.day_path(day)) {
            Ok(file) => file,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(unavailable(why)),
        };
        let len = file.metadata().map_err(unavailable)?.len();
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
/// read in place: the year's central directory once per day asked
/// (O(entries in the year zip), about three hundred), the day zip's own
/// directory once (O(its entries)), and a file by one local header and one
/// inflate. Nothing is extracted.
#[derive(Debug, Clone)]
pub struct NfoZips {
    dir: PathBuf,
}

impl NfoZips {
    /// The zips in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }
}

/// Where one member of an options day zip lies: the year zip, shared by
/// every entry of the day, and the member inside it.
#[derive(Debug)]
pub struct ZipFileLocator<B = std::fs::File> {
    file: Arc<B>,
    at: ZipLocator,
}

impl<B> Clone for ZipFileLocator<B> {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            at: self.at,
        }
    }
}

impl NfoSource for NfoZips {
    type Locator = ZipFileLocator;

    fn day(&self, day: Day) -> Result<Option<NfoDay<ZipFileLocator>>, CmRefusal> {
        let unavailable = |why: std::io::Error| CmRefusal::ArchiveUnavailable { kind: why.kind() };
        let path = self.dir.join(format!("{:04}.zip", day.year()));
        let file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(unavailable(why)),
        };
        let len = file.metadata().map_err(unavailable)?.len();
        listing_in(&Arc::new(file), len, day)
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
    let (base, span) = stored_span(&**file, &inner)?;
    let members = zip_entries(&**file, base, span)?;
    let mut listing = NfoDay::new(day, members.len());
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
    Ok(Some(listing))
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
    let mut rows = Vec::new();
    for (index, raw) in lines.enumerate() {
        let line = u32::try_from(index.saturating_add(2)).unwrap_or(u32::MAX);
        if rows.len() >= cap {
            return Err(NfoRefusal::RowsOverCap { cap });
        }
        let malformed = NfoRefusal::MalformedRow { line };
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let text = core::str::from_utf8(raw).map_err(|_| malformed.clone())?;
        let fields: Vec<&str> = text.split(',').collect();
        let &[
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
        ] = fields.as_slice()
        else {
            return Err(malformed);
        };
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
    Ok(NfoFile { rows })
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
/// The ticker refusals of [`decode_ticker`], the source's own, the
/// capital-market reader's `SourceLengthMismatch` / `SourceCrcMismatch`, and
/// every refusal of [`decode`]. A file that is not an option file of the day
/// is [`NfoRefusal::TickerUnparsed`].
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
