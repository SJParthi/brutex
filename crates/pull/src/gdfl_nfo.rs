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
//! # The contract a ticker names (FORMAT.md §6)
//!
//! `UNDERLYING DD MON YY STRIKE CE|PE` (`BANKNIFTY24FEB2651500PE`), and a
//! monthly 2018–2019 form `UNDERLYING YY MON STRIKE CE|PE` (`ACC18OCT1280PE`)
//! that states no expiry DAY. The rule is the format's own: the first form is
//! tried with the expiry a real weekday between the trade date and the trade
//! date plus 2,200 days; exactly one such reading is the contract, two are
//! refused as ambiguous, and none falls to the second form, which is refused
//! by name because its expiry day is not in the name and no sourced monthly
//! expiry calendar for 2018–2019 is recorded here (`docs/05-decisions.md`
//! D-2806 names that missing fact).
//!
//! **The two forms overlap, and the trade day chooses between them.** A
//! monthly name whose strike begins with two digits also reads as the dated
//! form, the month's year taken as the expiry DAY and the strike's first two
//! digits as the YEAR: `ADANIENT18DEC195CE` traded on 2018-12-03 is December
//! 2018, strike 195, and also 2019-12-18, strike 5. Taking the dated reading
//! filed such names under a wrong contract silently (D-3160). The rule is the
//! census of the operator's GDFL options tree ([`DATED_FORM_FROM`]): before
//! trade day 2019-02-01 only the weeklies of the underlyings in
//! [`DATED_BEFORE_CUTOVER`] use the dated form (from
//! [`INDEX_WEEKLY_DATED_FROM`], 2018-09-03) and every other name is the
//! monthly form; from 2019-02-01 every name is the dated form. So before the
//! cutover a name of any other underlying is read as the monthly form only;
//! a name of those two is read both ways and refused as
//! [`NfoRefusal::FormsAmbiguous`] if both readings name a contract alive on
//! the trade day (the census found none, and it stays loud); from the
//! cutover only the dated form is read, and a dated shape whose date is not a
//! weekday inside the window is [`NfoRefusal::ExpiryRefused`] (D-3164). A
//! strike's whole part never begins with `0` unless it is `0` itself (`0.5`),
//! which also keeps `2005` from reading as year 20, strike 5, and its
//! fraction never ends in `0` (`107.50`): the census saw neither (D-3161). The underlying may hold any byte a
//! `Symbol` admits (`M&M`, `NAM-INDIA`, `360ONE`); the strike may be decimal
//! (`107.5`, 10,750 paisa). An expiry the ticker states on an exchange holiday
//! is kept as the vendor stated it: the name is the contract's identity, and
//! moving it would file two names under one path.
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
//! two hash probes (expected O(1), measured by `C-GI-02` in
//! `crates/pull/benches/ratio.rs`). [`decode_ticker`] is O(ticker length),
//! bounded by the 64-byte cap it refuses past. [`decode`] is O(bytes).
//! Fetching a file is O(its compressed and rebuilt bytes). The listing and
//! the fetch are limits in `docs/06-limits.md`, not constant-time claims.

use std::collections::{HashMap, HashSet};
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

/// The first trade day `(year, month, day)` on which every GDFL option name
/// is the dated form `DD MON YY STRIKE`; before it only the weeklies of
/// [`DATED_BEFORE_CUTOVER`] are, and every other name is the monthly form
/// `YY MON STRIKE` (D-3160). Source: the census of the operator's GDFL
/// options tree relayed 2026-10-04 — all 20.9M names read under this rule,
/// none read two ways.
pub const DATED_FORM_FROM: (u16, u8, u8) = (2019, 2, 1);

/// The underlyings whose names may be the dated form before
/// [`DATED_FORM_FROM`] (their weeklies), by the same census (D-3160).
pub const DATED_BEFORE_CUTOVER: [&str; 2] = ["NIFTY", "BANKNIFTY"];

/// The first trade day `(year, month, day)` on which a weekly of
/// [`DATED_BEFORE_CUTOVER`] is in the dated form; before it no name is read
/// as dated. Source: the same census, relayed 2026-10-04 (D-3160).
pub const INDEX_WEEKLY_DATED_FROM: (u16, u8, u8) = (2018, 9, 3);

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
    /// Two tickers of one day name one contract (`100` and `100.00`, say),
    /// so their bars would file under one path: both refused.
    TickerAmbiguous {
        /// The ticker.
        ticker: String,
    },
    /// The ticker is the monthly 2018–2019 form, whose expiry day is not in
    /// the name; no sourced monthly expiry calendar is recorded (D-2806).
    MonthlyExpiryUnstated {
        /// The ticker.
        ticker: String,
    },
    /// Before the dated-form cutover an index ticker reads both as a dated
    /// contract and as a monthly one alive on the trade day, and the name
    /// alone cannot choose (D-3160).
    FormsAmbiguous {
        /// The ticker.
        ticker: String,
    },
    /// From the dated-form cutover the ticker has the dated shape, but its
    /// `DD MON YY` is not a real weekday between the trade day and the
    /// horizon (D-3164).
    ExpiryRefused {
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
    /// row (LTQ > 0) priced below one tick.
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
                "{ticker}: another ticker of the same day names the same contract; both refused rather than merged"
            ),
            Self::MonthlyExpiryUnstated { ticker } => write!(
                f,
                "{ticker}: the monthly form states no expiry day and no sourced monthly expiry calendar is recorded (D-2806)"
            ),
            Self::FormsAmbiguous { ticker } => write!(
                f,
                "{ticker}: before the dated-form cutover the name reads both as a dated and as a monthly contract alive on the trade day; refused rather than guessed (D-3160)"
            ),
            Self::ExpiryRefused { ticker } => write!(
                f,
                "{ticker}: the stated expiry is not a real weekday from the trade day to {EXPIRY_HORIZON_DAYS} days after it"
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
                "line {line}: the LTP is not a two-place decimal, or a traded row is below one tick"
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

/// The contract one option ticker names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionTicker {
    /// The underlying's symbol, exactly as the ticker spells it.
    pub underlying: Symbol,
    /// The store's contract segment.
    pub contract: Contract,
}

/// Whether `day` is a weekday. 1970-01-01, day 0, was a Thursday.
const fn is_weekday(day: Day) -> bool {
    (day.days_from_epoch() + 3) % 7 < 5
}

/// The month number a three-letter upper-case month name states.
fn month_of(name: &[u8]) -> Option<u8> {
    MONTHS
        .iter()
        .position(|m| m.as_bytes() == name)
        .and_then(|at| u8::try_from(at + 1).ok())
}

/// Two ASCII digits as a number.
fn two_digits(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [a, b] if a.is_ascii_digit() && b.is_ascii_digit() => Some((a - b'0') * 10 + (b - b'0')),
        _ => None,
    }
}

/// A positive strike in paisa: digits, optionally a dot and one or two
/// digits. The whole part never begins with `0` unless it is `0`, and a
/// fraction never ends in `0` (`100.0`, `107.50`): each is a second spelling
/// of a strike the census never saw (D-3161).
fn strike_paisa(text: &[u8]) -> Option<i64> {
    let text = core::str::from_utf8(text).ok()?;
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let shaped = !whole.is_empty()
        && (whole == "0" || !whole.starts_with('0'))
        && whole.bytes().all(|b| b.is_ascii_digit())
        && (!text.contains('.')
            || (!frac.is_empty()
                && !frac.ends_with('0')
                && frac.bytes().all(|b| b.is_ascii_digit())));
    if !shaped {
        return None;
    }
    crate::csv::paisa(text).filter(|&paisa| paisa > 0)
}

/// Whether `body` (a ticker without its side) reads as the monthly form
/// `YY MON STRIKE` at some split; with `window`, only a month holding at
/// least one day of `[from, to]` (days from the epoch) counts.
fn monthly_reading(body: &[u8], window: Option<(u32, u32)>) -> bool {
    (1..body.len()).any(|at| {
        let Some(rest) = body.get(at..) else {
            return false;
        };
        let (Some(yy), Some(month)) = (
            rest.get(0..2).and_then(two_digits),
            rest.get(2..5).and_then(month_of),
        ) else {
            return false;
        };
        rest.get(5..).and_then(strike_paisa).is_some()
            && window.is_none_or(|(from, to)| {
                Day::new(2000 + u16::from(yy), month, 1).is_ok_and(|first| {
                    first.end_of_month().days_from_epoch() >= from && first.days_from_epoch() <= to
                })
            })
    })
}

/// The contract `ticker`, a file stem without `.NFO`, names for a file of
/// trade day `trade`, by FORMAT.md §6 and the monthly-era rule (D-3160).
///
/// # Errors
///
/// [`NfoRefusal::TickerUnparsed`],
/// [`NfoRefusal::MonthlyExpiryUnstated`], [`NfoRefusal::FormsAmbiguous`],
/// [`NfoRefusal::UnderlyingRefused`] and
/// [`NfoRefusal::ContractUnrenderable`].
pub fn decode_ticker(ticker: &str, trade: Day) -> Result<OptionTicker, NfoRefusal> {
    let unparsed = || NfoRefusal::TickerUnparsed {
        ticker: ticker.to_owned(),
    };
    if ticker.len() > TICKER_CAP {
        return Err(unparsed());
    }
    let bytes = ticker.as_bytes();
    let (body, side) = match bytes.split_last_chunk::<2>() {
        Some((body, b"CE")) => (body, OptionSide::Call),
        Some((body, b"PE")) => (body, OptionSide::Put),
        _ => return Err(unparsed()),
    };
    let latest = trade.days_from_epoch().saturating_add(EXPIRY_HORIZON_DAYS);
    // AT MOST ONE first-form reading exists, so the first is the only one.
    // A reading needs everything after its `DD MON YY` to be a strike, which
    // is digits and at most one dot; a later reading's `MON` is three
    // letters inside that strike, which no strike holds, and an earlier one's
    // is the same argument mirrored. So a ticker cannot name two contracts by
    // itself; two TICKERS of one day naming one contract is refused by
    // `crate::gdfl_import` (`TickerAmbiguous`).
    // `tests::no_ticker_reads_as_two_contracts` walks every split of a sweep
    // of shapes to hold this.
    // The dated shape, its date not yet judged: (split, dd, mon, yy, strike).
    let shaped = (1..body.len()).find_map(|at| {
        let rest = body.get(at..)?;
        let dd = rest.get(0..2).and_then(two_digits)?;
        let mon = rest.get(2..5).and_then(month_of)?;
        let yy = rest.get(5..7).and_then(two_digits)?;
        let strike = rest.get(7..).and_then(strike_paisa)?;
        Some((at, dd, mon, yy, strike))
    });
    let first_form = shaped.and_then(|(at, dd, mon, yy, strike)| {
        let expiry = Day::new(2000 + u16::from(yy), mon, dd).ok()?;
        (is_weekday(expiry)
            && expiry.days_from_epoch() >= trade.days_from_epoch()
            && expiry.days_from_epoch() <= latest)
            .then_some((at, expiry, strike))
    });
    // The era rule (D-3160): which forms the trade day admits.
    let on = (trade.year(), trade.month(), trade.day());
    let before = on < DATED_FORM_FROM;
    let chosen = if before {
        let dated = first_form.filter(|&(at, ..)| {
            on >= INDEX_WEEKLY_DATED_FROM
                && ticker
                    .get(..at)
                    .is_some_and(|under| DATED_BEFORE_CUTOVER.contains(&under))
        });
        if dated.is_some() && monthly_reading(body, Some((trade.days_from_epoch(), latest))) {
            return Err(NfoRefusal::FormsAmbiguous {
                ticker: ticker.to_owned(),
            });
        }
        dated
    } else {
        first_form
    };
    let Some((at, expiry, strike)) = chosen else {
        // Before the cutover, the monthly form: YY MON STRIKE, no expiry day.
        // From the cutover, the dated shape with a date that is not one.
        return Err(if before && monthly_reading(body, None) {
            NfoRefusal::MonthlyExpiryUnstated {
                ticker: ticker.to_owned(),
            }
        } else if !before && shaped.is_some() {
            NfoRefusal::ExpiryRefused {
                ticker: ticker.to_owned(),
            }
        } else {
            unparsed()
        });
    };
    let name = ticker.get(..at).unwrap_or_default();
    let underlying = Symbol::new(name)
        .ok()
        .filter(|symbol| symbol.as_str() == name)
        .ok_or_else(|| NfoRefusal::UnderlyingRefused {
            ticker: ticker.to_owned(),
        })?;
    let unrenderable = || NfoRefusal::ContractUnrenderable {
        ticker: ticker.to_owned(),
    };
    let contract = Expiry::new(expiry.year(), expiry.month(), expiry.day())
        .ok()
        .and_then(|expiry| {
            Contract::of(Kind::Option {
                expiry,
                strike: Paisa::from_raw(strike),
                side,
            })
        })
        .ok_or_else(unrenderable)?;
    Ok(OptionTicker {
        underlying,
        contract,
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
    /// Every ticker named by more than one entry: a set, so [`Self::locate`]
    /// stays one probe however many duplicates a day holds (D-3162).
    duplicates: HashSet<Box<str>>,
    skipped: usize,
}

impl<L> NfoDay<L> {
    /// An empty listing of `day`.
    #[must_use]
    pub fn new(day: Day) -> Self {
        Self {
            day,
            folder: day_folder_name(day),
            entries: Vec::new(),
            by_ticker: HashMap::new(),
            duplicates: HashSet::new(),
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

    /// The file holding `ticker`, if the day has one: two hash probes, the
    /// duplicate set and the ticker map.
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
        let mut listing = NfoDay::new(day);
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
    let mut listing = NfoDay::new(day);
    for entry in zip_entries(&**file, base, span)? {
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
        if ltq > 0 && paisa < costs::rate::TICK.raw() {
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

#[cfg(test)]
#[path = "gdfl_nfo_attack_tests.rs"]
mod attack_tests;
