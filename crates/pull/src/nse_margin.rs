//! Each stock's dated intraday leverage, read from NSE's daily margin files
//! (FNO-LEVERAGE; D-8150 to D-8155; SPEC 11.1 and 11.2; OP-16, OP-17).
//!
//! # What is read (D-8150)
//!
//! NSE's `C_VAR1_DDMMYYYY_1.DAT`: the FIRST batch of each trading day's margin
//! file, the one SPEC 11.1 names ("batch 1 of D"; PARTS FNO-LEVERAGE, T02-02).
//! A file of any other batch is not read; the build names it in its
//! [`Report`]. Of each row whose series column holds [`SERIES`] (`EQ`), three
//! columns are kept, as the design gives them (PARTS FNO-LEVERAGE, citing
//! `sources-leverage-execution.md` section 1, items 1-2):
//!
//! * column 7 ([`VAR_COLUMN`]): the `VaR` margin;
//! * column 8 ([`ELM_COLUMN`]): the extreme loss margin (ELM);
//! * column 9 ([`EXTRA_COLUMN`]): NSE's additional margin.
//!
//! Each is a percentage of the position's value. It is held EXACTLY, as parts
//! per million of the value ([`PPM_PER_PERCENT`] to one percentage point), so
//! `12.50` is 125,000 and `22.11` is 221,100. No float is ever formed
//! (`CLAUDE.md` section 7). A number with more than [`FRACTION_DIGITS`]
//! fraction digits cannot be held exactly and is refused, never rounded.
//!
//! **UNVERIFIED in this build (D-8153).** The symbol in column 2
//! ([`SYMBOL_COLUMN`]) and the series in column 3 ([`SERIES_COLUMN`]) are this
//! build's reading of NSE's layout. The design names columns 7, 8 and 9 and
//! the `EQ` series and nothing else, and its source,
//! `sources-leverage-execution.md`, is on the operator's Mac only, so the two
//! positions are not traced to a recorded source here. A line whose third
//! column is not `EQ` (a header record, any other series) is counted and
//! ignored, never parsed further and never refused, so a header needs no
//! assumed shape. The Mac's operations step reads one real file before the
//! build verb runs; a layout that differs refuses loudly, by row and column,
//! because an `EQ` row this reader cannot hold is a [`ParseError`], never a
//! skip.
//!
//! # The leverage (D-8150)
//!
//! `M = VaR + ELM`, and `L = min(5, 100 / max(20, M))` (SPEC 11.1). [`leverage`]
//! returns it as an exact reduced [`Ratio`]: `1,000,000 / max(200,000, M)` in
//! parts per million. The cap at 5 IS the floor at 20%: `max(20, M) >= 20`
//! makes the quotient at most `100 / 20 = 5`, so one comparison carries both
//! halves of the formula (a compile-time assertion below checks the two
//! constants agree). `M = 25%` is `4/1`, `M = 22.11%` is `10000/2211` (exactly
//! `100 / 22.11`), and `M = 12.5%` caps at `5/1`.
//!
//! # Three readings (D-8152; SPEC 11.2, OP-16, OP-17)
//!
//! * [`Reading::Day`]: `M` from columns 7 and 8 of the trade day's own file;
//! * [`Reading::Today`]: `M` from columns 7 and 8 of TODAY's file (the second
//!   clock of operator rule 7);
//! * [`Reading::WithExtra`]: `M` from columns 7, 8 and 9 of the trade day's
//!   file, because whether Dhan counts NSE's additional margin is UNVERIFIED
//!   (T01-22).
//!
//! [`MarginTable::readings`] reads all three; the caller names the day AND
//! today, and the symbol each was listed under (a share renamed since the
//! trade day is listed under another symbol today; the caller supplies the
//! dated symbol). [`Readings::smallest`] is the one the quantity uses (SPEC
//! 11.2: valid under every reading). A missing today's file is
//! [`MarginUnknown::NoFile`] for today, never the newest file held.
//!
//! # The table (D-8151)
//!
//! `build_from_dir(src, out)` reads every batch-1 file in `src` and writes
//! `<out>/nse-margin-v1/`:
//!
//! * `symbols.nsd` ([`DICTIONARY`]): every `EQ` symbol any file listed, sorted,
//!   with the span of months held. Sorted, so the bytes depend on the SET of
//!   files and rows, not on the order the directory or a file lists them in.
//! * `<YYYY-MM>.nsm` ([`month_name`]), one per month of the span, a month with
//!   no file included: a header, then [`DAY_SLOTS`] day slots of one cell per
//!   dictionary symbol ([`CELL_LEN`] bytes: columns 7, 8, 9 as little-endian
//!   `u32` parts per million, all three [`ABSENT`] where the day's file did
//!   not list the symbol), then a BLAKE3 trailer. A bitmap in the header
//!   says which day slots had a file.
//!
//! Every file is sealed by its own BLAKE3 trailer, and every month header
//! carries the BLAKE3 of the dictionary file it was built against, so a set
//! mixed from two builds is refused by name ([`OpenError::Corrupt`]). Each
//! file is written whole to a hidden `.part` file and renamed into place; the
//! dictionary is written last. A build is a pure function of the batch-1
//! files: rebuilding over the same files writes the same bytes
//! (`CLAUDE.md` section 3 rule 5).
//!
//! # The lookup (D-8154)
//!
//! [`MarginTable::margin`] is O(1): one month subtraction, one bit of the
//! day bitmap, one symbol hash probe and one cell read at
//! `((month * 31 + day - 1) * symbols + symbol)`. It never reads another day:
//! a day outside the span, a day with no file, a symbol never listed and a
//! symbol the day's file did not list are each their own
//! [`MarginUnknown`]. C-FNO-LEV-01 measures it flat at 1x, 10x and 100x the
//! months held (`crates/pull/benches/ratio.rs`).
//!
//! # What this module does not do
//!
//! It opens no socket and reads no clock. Downloading NSE's files is an
//! operations step that needs the operator's yes (PARTS decision 9.3), and
//! the build verb is FNO-RUN's (`cli fno margin build`). [`MarginTable::file_digest`]
//! is the term FNO-SEC-WIRE's run identity calls `leverage_digest`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};

use crate::session::Day;

/// The first decision of this part's block (`docs/05-decisions.md`).
pub const DECISION: &str = "D-8150";
/// The directory the build writes under its `out` argument (PARTS
/// FNO-LEVERAGE: `reference/nse-margin-v1`).
pub const DIR_NAME: &str = "nse-margin-v1";
/// The month files' extension, without the dot.
pub const EXTENSION: &str = "nsm";
/// The symbol dictionary's file name inside [`DIR_NAME`].
pub const DICTIONARY: &str = "symbols.nsd";
/// A margin file's name starts with this ...
pub const SOURCE_PREFIX: &str = "C_VAR1_";
/// ... and ends with this; between them `DDMMYYYY_N`, `N` the batch.
pub const SOURCE_SUFFIX: &str = ".DAT";
/// The one batch read: the first of the trading day (SPEC 11.1).
pub const BATCH: u8 = 1;
/// The one series read.
pub const SERIES: &str = "EQ";
/// The symbol's column, counted from 1. UNVERIFIED here (D-8153).
pub const SYMBOL_COLUMN: usize = 2;
/// The series' column, counted from 1. UNVERIFIED here (D-8153).
pub const SERIES_COLUMN: usize = 3;
/// The `VaR` margin's column, counted from 1 (the design's column 7).
pub const VAR_COLUMN: usize = 7;
/// The extreme loss margin's column, counted from 1 (the design's column 8).
pub const ELM_COLUMN: usize = 8;
/// NSE's additional margin's column, counted from 1 (the design's column 9).
pub const EXTRA_COLUMN: usize = 9;
/// Parts per million in one percentage point.
pub const PPM_PER_PERCENT: u32 = 10_000;
/// The most fraction digits a percentage may carry and stay exact in parts
/// per million.
pub const FRACTION_DIGITS: usize = 4;
/// The margin floor, 20% (SPEC 11.1 and 11.3), in parts per million.
pub const FLOOR_PPM: u32 = 200_000;
/// The whole position, 100%, in parts per million.
pub const WHOLE_PPM: u32 = 1_000_000;
/// The intraday leverage cap (SPEC 11.1).
pub const MAX_LEVERAGE: u32 = 5;
/// A cell column the day's file did not list. A percentage this large is
/// refused by the parser, so it can never be read as a value.
pub const ABSENT: u32 = u32::MAX;
/// Day slots per month file: one per day of the longest month.
pub const DAY_SLOTS: usize = 31;
/// Bytes of one cell: columns 7, 8 and 9, a little-endian `u32` each.
pub const CELL_LEN: usize = 12;
/// Bytes of a BLAKE3 digest.
pub const DIGEST_LEN: usize = 32;
/// The first eight bytes of a month file.
pub const MONTH_MAGIC: [u8; 8] = *b"BRXNSM01";
/// The first eight bytes of the dictionary file.
pub const DICTIONARY_MAGIC: [u8; 8] = *b"BRXNSD01";
/// The one format version this module writes and reads.
pub const VERSION: u16 = 1;
/// Bytes of a month file's header.
pub const MONTH_HEADER_LEN: usize = 56;
/// Bytes of the dictionary file's header.
pub const DICTIONARY_HEADER_LEN: usize = 24;

// The cap is the floor: 100% over 20% is exactly 5.
const _: () = assert!(WHOLE_PPM / FLOOR_PPM == MAX_LEVERAGE && WHOLE_PPM.is_multiple_of(FLOOR_PPM));
// A month's day bitmap is one `u32`, and the encoder walks days 1 to 31.
const _: () = assert!(DAY_SLOTS == 31);

// ───────────────────────────────── parsing ─────────────────────────────────

/// One `EQ` row of a margin file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginRow {
    /// The symbol, as the file lists it (column 2).
    pub symbol: Symbol,
    /// The series the row was read under: always [`SERIES`], kept so a row
    /// printed alone says which series it is.
    pub series: &'static str,
    /// Column 7, the `VaR` margin, in parts per million.
    pub var_ppm: u32,
    /// Column 8, the extreme loss margin, in parts per million.
    pub elm_ppm: u32,
    /// Column 9, NSE's additional margin, in parts per million.
    pub extra_ppm: u32,
}

/// One file's `EQ` rows, in file order, and the rows of other series it
/// ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The `EQ` rows.
    pub rows: Vec<MarginRow>,
    /// Lines whose series column is not `EQ`, counted and not read further.
    pub other_series: usize,
}

/// A margin file refused, naming the line (counted from 1) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line is not UTF-8 text.
    NotText {
        /// The line.
        line: usize,
    },
    /// An empty line before the file's end.
    EmptyLine {
        /// The line.
        line: usize,
    },
    /// Too few comma-separated columns to read.
    TooFewColumns {
        /// The line.
        line: usize,
        /// Columns found.
        found: usize,
        /// Columns needed.
        needed: usize,
    },
    /// Column 2 of an `EQ` row is not a symbol.
    BadSymbol {
        /// The line.
        line: usize,
        /// The text read.
        text: String,
    },
    /// A margin column of an `EQ` row is not a non-negative percentage with
    /// at most [`FRACTION_DIGITS`] fraction digits below the [`ABSENT`] bound.
    BadNumber {
        /// The line.
        line: usize,
        /// The column, counted from 1.
        column: usize,
        /// The text read.
        text: String,
    },
    /// A second `EQ` row for one symbol.
    DuplicateSymbol {
        /// The line of the second row.
        line: usize,
        /// The symbol.
        symbol: Symbol,
        /// The line of the first row.
        first: usize,
    },
    /// No `EQ` row at all: an empty or truncated download, never a day on
    /// which nothing was listed.
    NoEqRows,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotText { line } => write!(f, "line {line} is not UTF-8 text"),
            Self::EmptyLine { line } => write!(f, "line {line} is empty before the file's end"),
            Self::TooFewColumns {
                line,
                found,
                needed,
            } => write!(f, "line {line} has {found} columns, {needed} needed"),
            Self::BadSymbol { line, text } => {
                write!(f, "line {line}: column {SYMBOL_COLUMN} {text:?} is not a symbol")
            }
            Self::BadNumber { line, column, text } => write!(
                f,
                "line {line}: column {column} {text:?} is not a percentage held exactly"
            ),
            Self::DuplicateSymbol {
                line,
                symbol,
                first,
            } => write!(f, "line {line}: a second {SERIES} row for {symbol} (first at line {first})"),
            Self::NoEqRows => write!(f, "the file lists no {SERIES} row"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses one margin file: its `EQ` rows, columns 7, 8 and 9.
///
/// # Errors
///
/// A [`ParseError`] naming the first line refused: malformed rows are refused
/// by name, never skipped. A line of another series is never refused.
pub fn parse_c_var1(bytes: &[u8]) -> Result<Vec<MarginRow>, ParseError> {
    parse_counted(bytes).map(|parsed| parsed.rows)
}

/// [`parse_c_var1`], with the count of other-series lines it ignored.
///
/// # Errors
///
/// As [`parse_c_var1`].
pub fn parse_counted(bytes: &[u8]) -> Result<Parsed, ParseError> {
    let mut rows = Vec::new();
    let mut other_series = 0;
    let mut seen: HashMap<Symbol, usize> = HashMap::new();
    let lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    let last = lines.len();
    for (at, raw) in lines.into_iter().enumerate() {
        let line = at + 1;
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        if raw.is_empty() {
            if line == last {
                continue;
            }
            return Err(ParseError::EmptyLine { line });
        }
        let text = std::str::from_utf8(raw).map_err(|_| ParseError::NotText { line })?;
        let fields: Vec<&str> = text.split(',').map(str::trim).collect();
        if fields.len() < SERIES_COLUMN {
            return Err(too_few(line, fields.len(), SERIES_COLUMN));
        }
        if field(&fields, SERIES_COLUMN) != SERIES {
            other_series += 1;
            continue;
        }
        if fields.len() < EXTRA_COLUMN {
            return Err(too_few(line, fields.len(), EXTRA_COLUMN));
        }
        let row = eq_row(line, &fields)?;
        if let Some(first) = seen.insert(row.symbol, line) {
            return Err(ParseError::DuplicateSymbol {
                line,
                symbol: row.symbol,
                first,
            });
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(ParseError::NoEqRows);
    }
    Ok(Parsed { rows, other_series })
}

const fn too_few(line: usize, found: usize, needed: usize) -> ParseError {
    ParseError::TooFewColumns {
        line,
        found,
        needed,
    }
}

/// Column `column` (from 1), present once the caller has checked the count.
fn field<'a>(fields: &[&'a str], column: usize) -> &'a str {
    fields.get(column - 1).copied().unwrap_or("")
}

/// The `EQ` row on `line`, its columns already counted.
fn eq_row(line: usize, fields: &[&str]) -> Result<MarginRow, ParseError> {
    let text = field(fields, SYMBOL_COLUMN);
    let symbol = Symbol::new(text).map_err(|_| ParseError::BadSymbol {
        line,
        text: text.to_owned(),
    })?;
    let number = |column: usize| {
        let text = field(fields, column);
        percent_ppm(text).ok_or_else(|| ParseError::BadNumber {
            line,
            column,
            text: text.to_owned(),
        })
    };
    Ok(MarginRow {
        symbol,
        series: SERIES,
        var_ppm: number(VAR_COLUMN)?,
        elm_ppm: number(ELM_COLUMN)?,
        extra_ppm: number(EXTRA_COLUMN)?,
    })
}

/// Whether `text` is one or more ASCII digits.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A percentage in parts per million: ASCII digits, then optionally `.` and
/// one to [`FRACTION_DIGITS`] digits; below [`ABSENT`].
fn percent_ppm(text: &str) -> Option<u32> {
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text, None), |(whole, fraction)| (whole, Some(fraction)));
    if !digits(whole) || fraction.is_some_and(|f| !digits(f) || f.len() > FRACTION_DIGITS) {
        return None;
    }
    let mut ppm: u64 = 0;
    for b in whole.bytes() {
        ppm = ppm.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    ppm = ppm.checked_mul(u64::from(PPM_PER_PERCENT))?;
    let mut place = PPM_PER_PERCENT;
    for b in fraction.unwrap_or_default().bytes() {
        place /= 10;
        ppm = ppm.checked_add(u64::from(b - b'0') * u64::from(place))?;
    }
    u32::try_from(ppm).ok().filter(|&value| value < ABSENT)
}

// ─────────────────────────────── source names ──────────────────────────────

/// What a margin file's name says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceFile {
    /// The trading day, from `DDMMYYYY`.
    pub day: Day,
    /// The batch, from `N`.
    pub batch: u8,
}

/// Reads `C_VAR1_DDMMYYYY_N.DAT`.
///
/// `Ok(None)` for a name that does not start with [`SOURCE_PREFIX`]: not a
/// margin file at all.
///
/// # Errors
///
/// [`BuildError::BadSourceName`] for a name that starts with the prefix and
/// does not continue as `DDMMYYYY_N.DAT` with a real day: a file claiming to
/// be a margin file is never passed over in silence.
pub fn source_file(name: &str) -> Result<Option<SourceFile>, BuildError> {
    let Some(rest) = name.strip_prefix(SOURCE_PREFIX) else {
        return Ok(None);
    };
    let bad = || BuildError::BadSourceName {
        name: name.to_owned(),
    };
    let stem = rest.strip_suffix(SOURCE_SUFFIX).unwrap_or("");
    let (date, batch) = stem.split_once('_').unwrap_or(("", ""));
    if date.len() != 8 || !digits(date) || !digits(batch) {
        return Err(bad());
    }
    let number = |from: usize, to: usize| {
        date.get(from..to)
            .and_then(|text| text.parse::<u16>().ok())
            .unwrap_or(0)
    };
    let small = |value: u16| u8::try_from(value).unwrap_or(0);
    let day = Day::new(number(4, 8), small(number(2, 4)), small(number(0, 2))).map_err(|_| bad())?;
    let batch = batch.parse::<u8>().map_err(|_| bad())?;
    Ok(Some(SourceFile { day, batch }))
}

// ─────────────────────────────────── build ─────────────────────────────────

/// A build refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// A path could not be read.
    Read {
        /// The path.
        path: PathBuf,
        /// The operating system's words.
        why: String,
    },
    /// A path could not be written.
    Write {
        /// The path.
        path: PathBuf,
        /// The operating system's words.
        why: String,
    },
    /// A name starting with [`SOURCE_PREFIX`] that is not a margin file's.
    BadSourceName {
        /// The name.
        name: String,
    },
    /// A margin file refused by the parser.
    Parse {
        /// The file's name.
        name: String,
        /// Why.
        error: ParseError,
    },
    /// No batch-1 margin file was read.
    NoDays,
    /// Two inputs for one day, or days out of order.
    DayOrder {
        /// The day.
        day: Day,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, why } => write!(f, "cannot read {}: {why}", path.display()),
            Self::Write { path, why } => write!(f, "cannot write {}: {why}", path.display()),
            Self::BadSourceName { name } => {
                write!(f, "{name:?} starts as a margin file and is not {SOURCE_PREFIX}DDMMYYYY_N{SOURCE_SUFFIX}")
            }
            Self::Parse { name, error } => write!(f, "{name}: {error}"),
            Self::NoDays => write!(f, "no batch-{BATCH} margin file was read"),
            Self::DayOrder { day } => write!(f, "{day} is given twice or out of order"),
        }
    }
}

impl std::error::Error for BuildError {}

fn read_error(path: &Path, why: &std::io::Error) -> BuildError {
    BuildError::Read {
        path: path.to_path_buf(),
        why: why.to_string(),
    }
}

fn write_error(path: &Path, why: &std::io::Error) -> BuildError {
    BuildError::Write {
        path: path.to_path_buf(),
        why: why.to_string(),
    }
}

/// What a build read and wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The table's directory, `<out>/nse-margin-v1`.
    pub dir: PathBuf,
    /// Batch-1 files read, one per trading day.
    pub files_read: usize,
    /// `EQ` rows read.
    pub rows: usize,
    /// Lines of other series, counted and ignored.
    pub other_series: usize,
    /// Symbols in the dictionary.
    pub symbols: usize,
    /// The first day read.
    pub first_day: Day,
    /// The last day read.
    pub last_day: Day,
    /// Month files written, months with no file included.
    pub months: usize,
    /// Margin files of another batch, not read, by name.
    pub other_batches: Vec<String>,
    /// Entries that are not margin files at all, not read, by name.
    pub not_source: Vec<String>,
    /// [`MarginTable::file_digest_hex`] of what was written.
    pub digest_hex: String,
}

/// The bytes a build writes, before any is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The dictionary file.
    pub dictionary: Vec<u8>,
    /// Each month file's name and bytes, in month order.
    pub months: Vec<(String, Vec<u8>)>,
    /// Symbols in the dictionary.
    pub symbols: usize,
    /// The first day encoded.
    pub first_day: Day,
    /// The last day encoded.
    pub last_day: Day,
}

impl Encoded {
    /// The month files' bytes, in month order, as
    /// [`MarginTable::from_bytes`] takes them.
    #[must_use]
    pub fn month_bytes(&self) -> Vec<&[u8]> {
        self.months.iter().map(|(_, bytes)| bytes.as_slice()).collect()
    }
}

/// A month file's name: `<YYYY-MM>.nsm`.
#[must_use]
pub fn month_name(year: u16, month: u8) -> String {
    format!("{year:04}-{month:02}.{EXTENSION}")
}

/// `year * 12 + month - 1`: months counted from year 0.
fn month_index(year: u16, month: u8) -> usize {
    usize::from(year) * 12 + usize::from(month) - 1
}

/// The `(year, month)` of a [`month_index`] (from a `u16` year, so it fits).
fn month_of(index: usize) -> (u16, u8) {
    (
        u16::try_from(index / 12).unwrap_or(u16::MAX),
        u8::try_from(index % 12 + 1).unwrap_or(u8::MAX),
    )
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// `bytes` followed by its own BLAKE3.
fn seal(mut bytes: Vec<u8>) -> Vec<u8> {
    let digest = brutex_core::blake3::hash(&bytes);
    bytes.extend_from_slice(&digest);
    bytes
}

/// The last [`DIGEST_LEN`] bytes of a sealed file.
fn trailer(bytes: &[u8]) -> &[u8] {
    bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..).unwrap_or(&[])
}

/// BLAKE3 over the dictionary's trailer and each month's, in month order:
/// one digest for the whole set, the term a run's identity records.
fn set_digest(dictionary: &[u8], months: &[&[u8]]) -> [u8; DIGEST_LEN] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(trailer(dictionary));
    for month in months {
        hasher.update(trailer(month));
    }
    hasher.finalize()
}

fn hex(digest: &[u8]) -> String {
    digest.iter().fold(String::new(), |mut out, byte| {
        let _ = fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}"));
        out
    })
}

/// Encodes one batch-1 file's rows per day into the table's bytes. Pure: the
/// same days give the same bytes, whatever order each day's rows are in.
///
/// # Errors
///
/// [`BuildError::NoDays`] for no day, and [`BuildError::DayOrder`] unless the
/// days are strictly ascending.
pub fn encode(days: &[(Day, Vec<MarginRow>)]) -> Result<Encoded, BuildError> {
    let (Some((first, _)), Some((last, _))) = (days.first(), days.last()) else {
        return Err(BuildError::NoDays);
    };
    for ((before, _), (after, _)) in days.iter().zip(days.iter().skip(1)) {
        if before >= after {
            return Err(BuildError::DayOrder { day: *after });
        }
    }
    let names: BTreeSet<Symbol> = days
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.symbol))
        .collect();
    let index: HashMap<Symbol, usize> = names.iter().enumerate().map(|(i, s)| (*s, i)).collect();
    let mut dictionary = Vec::with_capacity(DICTIONARY_HEADER_LEN + names.len() * SYMBOL_CAPACITY);
    dictionary.extend_from_slice(&DICTIONARY_MAGIC);
    put_u16(&mut dictionary, VERSION);
    put_u16(&mut dictionary, first.year());
    dictionary.extend_from_slice(&[first.month(), 0]);
    put_u16(&mut dictionary, last.year());
    dictionary.extend_from_slice(&[last.month(), 0, 0, 0]);
    put_u32(&mut dictionary, u32::try_from(names.len()).unwrap_or(u32::MAX));
    for name in &names {
        let mut padded = [0u8; SYMBOL_CAPACITY];
        for (slot, byte) in padded.iter_mut().zip(name.as_str().bytes()) {
            *slot = byte;
        }
        dictionary.extend_from_slice(&padded);
    }
    let dictionary = seal(dictionary);
    let bound = brutex_core::blake3::hash(&dictionary);
    let mut by_month: BTreeMap<usize, BTreeMap<u8, HashMap<usize, [u32; 3]>>> = BTreeMap::new();
    for (day, rows) in days {
        let cells = rows
            .iter()
            .map(|row| {
                let at = index.get(&row.symbol).copied().unwrap_or(usize::MAX);
                (at, [row.var_ppm, row.elm_ppm, row.extra_ppm])
            })
            .collect();
        by_month
            .entry(month_index(day.year(), day.month()))
            .or_default()
            .insert(day.day(), cells);
    }
    let empty = BTreeMap::new();
    let months = (month_index(first.year(), first.month())..=month_index(last.year(), last.month()))
        .map(|at| {
            let (year, month) = month_of(at);
            let held = by_month.get(&at).unwrap_or(&empty);
            (month_name(year, month), month_file(year, month, names.len(), &bound, held))
        })
        .collect();
    Ok(Encoded {
        dictionary,
        months,
        symbols: names.len(),
        first_day: *first,
        last_day: *last,
    })
}

/// One month file's bytes: header, day slots, trailer.
fn month_file(
    year: u16,
    month: u8,
    symbols: usize,
    bound: &[u8; DIGEST_LEN],
    held: &BTreeMap<u8, HashMap<usize, [u32; 3]>>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(MONTH_HEADER_LEN + DAY_SLOTS * symbols * CELL_LEN + DIGEST_LEN);
    out.extend_from_slice(&MONTH_MAGIC);
    put_u16(&mut out, VERSION);
    put_u16(&mut out, year);
    out.extend_from_slice(&[month, 0, 0, 0]);
    put_u32(&mut out, u32::try_from(symbols).unwrap_or(u32::MAX));
    let bitmap = held.keys().fold(0u32, |bits, day| bits | (1 << (day - 1)));
    put_u32(&mut out, bitmap);
    out.extend_from_slice(bound);
    for day in 1..=31u8 {
        let cells = held.get(&day);
        for symbol in 0..symbols {
            let cell = cells
                .and_then(|row| row.get(&symbol))
                .copied()
                .unwrap_or([ABSENT; 3]);
            for value in cell {
                put_u32(&mut out, value);
            }
        }
    }
    seal(out)
}

/// Every name in `src`, split: batch-1 margin files by day, other batches,
/// and entries that are not margin files.
#[derive(Default)]
struct Listing {
    sources: Vec<(Day, PathBuf, String)>,
    other_batches: Vec<String>,
    not_source: Vec<String>,
}

fn list_sources(src: &Path) -> Result<Listing, BuildError> {
    let mut listing = Listing::default();
    for entry in fs::read_dir(src).map_err(|e| read_error(src, &e))? {
        let entry = entry.map_err(|e| read_error(src, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        match source_file(&name)? {
            None => listing.not_source.push(name),
            Some(file) if file.batch == BATCH => listing.sources.push((file.day, entry.path(), name)),
            Some(_) => listing.other_batches.push(name),
        }
    }
    listing.sources.sort();
    listing.other_batches.sort();
    listing.not_source.sort();
    Ok(listing)
}

/// Writes `bytes` to `<dir>/<name>` through a hidden `.part` file and a
/// rename, so a reader never meets half a file.
fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), BuildError> {
    let path = dir.join(name);
    let part = dir.join(format!(".{name}.part"));
    fs::write(&part, bytes).map_err(|e| write_error(&part, &e))?;
    fs::rename(&part, &path).map_err(|e| write_error(&path, &e))
}

/// Reads every batch-1 margin file in `src` and writes the dated table to
/// `<out>/nse-margin-v1/`: the month files first, the dictionary last. No
/// network I/O: the files are already on disk.
///
/// # Errors
///
/// A [`BuildError`] naming the path, the file or the day refused. Nothing
/// is skipped in silence: other batches and other entries are named in the
/// [`Report`].
pub fn build_from_dir(src: &Path, out: &Path) -> Result<Report, BuildError> {
    let listing = list_sources(src)?;
    let mut days = Vec::with_capacity(listing.sources.len());
    let (mut rows, mut other_series) = (0, 0);
    for (day, path, name) in &listing.sources {
        let bytes = fs::read(path).map_err(|e| read_error(path, &e))?;
        let parsed = parse_counted(&bytes).map_err(|error| BuildError::Parse {
            name: name.clone(),
            error,
        })?;
        rows += parsed.rows.len();
        other_series += parsed.other_series;
        days.push((*day, parsed.rows));
    }
    let encoded = encode(&days)?;
    let dir = out.join(DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| write_error(&dir, &e))?;
    for (name, bytes) in &encoded.months {
        write_file(&dir, name, bytes)?;
    }
    write_file(&dir, DICTIONARY, &encoded.dictionary)?;
    let digest = set_digest(&encoded.dictionary, &encoded.month_bytes());
    Ok(Report {
        dir,
        files_read: days.len(),
        rows,
        other_series,
        symbols: encoded.symbols,
        first_day: encoded.first_day,
        last_day: encoded.last_day,
        months: encoded.months.len(),
        other_batches: listing.other_batches,
        not_source: listing.not_source,
        digest_hex: hex(&digest),
    })
}

// ────────────────────────────────── reading ────────────────────────────────

/// A table refused at open, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// A file could not be read.
    Read {
        /// The path.
        path: PathBuf,
        /// The operating system's words.
        why: String,
    },
    /// A file's bytes failed a check.
    Corrupt {
        /// The file's name.
        file: String,
        /// The check that failed.
        what: &'static str,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, why } => write!(f, "cannot read {}: {why}", path.display()),
            Self::Corrupt { file, what } => write!(f, "{file} is refused: {what}"),
        }
    }
}

impl std::error::Error for OpenError {}

fn corrupt(file: &str, what: &'static str) -> OpenError {
    OpenError::Corrupt {
        file: file.to_owned(),
        what,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    let mut word = [0u8; 2];
    for (slot, byte) in word.iter_mut().zip(bytes.iter().skip(at)) {
        *slot = *byte;
    }
    u16::from_le_bytes(word)
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    let mut word = [0u8; 4];
    for (slot, byte) in word.iter_mut().zip(bytes.iter().skip(at)) {
        *slot = *byte;
    }
    u32::from_le_bytes(word)
}

fn byte_at(bytes: &[u8], at: usize) -> u8 {
    bytes.get(at).copied().unwrap_or(0)
}

/// Checks a sealed file's length floor, trailer, magic and version.
fn check_seal(file: &str, bytes: &[u8], header: usize, magic: [u8; 8]) -> Result<(), OpenError> {
    if bytes.len() < header + DIGEST_LEN {
        return Err(corrupt(file, "shorter than its header and digest"));
    }
    let body = bytes.get(..bytes.len() - DIGEST_LEN).unwrap_or(&[]);
    if brutex_core::blake3::hash(body).as_slice() != trailer(bytes) {
        return Err(corrupt(file, "its digest does not match its bytes"));
    }
    if bytes.get(..8) != Some(magic.as_slice()) {
        return Err(corrupt(file, "wrong magic"));
    }
    if u16_at(bytes, 8) != VERSION {
        return Err(corrupt(file, "unknown format version"));
    }
    Ok(())
}

/// The dictionary's span and names.
struct Dictionary {
    first: usize,
    last: usize,
    names: Vec<Symbol>,
}

fn read_dictionary(bytes: &[u8]) -> Result<Dictionary, OpenError> {
    check_seal(DICTIONARY, bytes, DICTIONARY_HEADER_LEN, DICTIONARY_MAGIC)?;
    let count = usize::try_from(u32_at(bytes, 20)).unwrap_or(usize::MAX);
    let expected = count
        .checked_mul(SYMBOL_CAPACITY)
        .and_then(|names| names.checked_add(DICTIONARY_HEADER_LEN + DIGEST_LEN));
    if expected != Some(bytes.len()) {
        return Err(corrupt(DICTIONARY, "its length is not its symbol count's"));
    }
    let (first_month, last_month) = (byte_at(bytes, 12), byte_at(bytes, 16));
    if !(1..=12).contains(&first_month) || !(1..=12).contains(&last_month) {
        return Err(corrupt(DICTIONARY, "a span month outside 1 to 12"));
    }
    let first = month_index(u16_at(bytes, 10), first_month);
    let last = month_index(u16_at(bytes, 14), last_month);
    if first > last {
        return Err(corrupt(DICTIONARY, "its span ends before it starts"));
    }
    let body = bytes
        .get(DICTIONARY_HEADER_LEN..bytes.len() - DIGEST_LEN)
        .unwrap_or(&[]);
    let mut names = Vec::with_capacity(count);
    for padded in body.chunks_exact(SYMBOL_CAPACITY) {
        let used = padded.iter().position(|&b| b == 0).unwrap_or(SYMBOL_CAPACITY);
        let (text, tail) = padded.split_at_checked(used).unwrap_or((padded, &[]));
        let name = std::str::from_utf8(text)
            .ok()
            .and_then(|text| Symbol::new(text).ok())
            .filter(|name| name.as_str().as_bytes() == text)
            .filter(|_| tail.iter().all(|&b| b == 0));
        let Some(name) = name else {
            return Err(corrupt(DICTIONARY, "a name that is not a symbol as written"));
        };
        if names.last().is_some_and(|before| *before >= name) {
            return Err(corrupt(DICTIONARY, "names not strictly sorted"));
        }
        names.push(name);
    }
    Ok(Dictionary { first, last, names })
}

/// One month file checked against its place in the span; its day bitmap.
fn read_month(
    file: &str,
    bytes: &[u8],
    at: usize,
    symbols: usize,
    bound: &[u8; DIGEST_LEN],
) -> Result<u32, OpenError> {
    let expected = symbols
        .checked_mul(DAY_SLOTS * CELL_LEN)
        .and_then(|cells| cells.checked_add(MONTH_HEADER_LEN + DIGEST_LEN));
    if expected != Some(bytes.len()) {
        return Err(corrupt(file, "its length is not the dictionary's stride"));
    }
    check_seal(file, bytes, MONTH_HEADER_LEN, MONTH_MAGIC)?;
    let (year, month) = month_of(at);
    if u16_at(bytes, 10) != year || byte_at(bytes, 12) != month {
        return Err(corrupt(file, "its header names another month"));
    }
    if usize::try_from(u32_at(bytes, 16)).ok() != Some(symbols) {
        return Err(corrupt(file, "its symbol count is not the dictionary's"));
    }
    if bytes.get(24..24 + DIGEST_LEN) != Some(bound.as_slice()) {
        return Err(corrupt(file, "it was built against another dictionary"));
    }
    let held = u32_at(bytes, 20);
    let days = Day::new(year, month, 1).map_or(0, |day| day.end_of_month().day());
    if held.checked_shr(u32::from(days)).unwrap_or(0) != 0 {
        return Err(corrupt(file, "a held day past the month's end"));
    }
    Ok(held)
}

/// The three margin columns of one stock on one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margin {
    /// The day whose file it was read from.
    pub day: Day,
    /// Column 7, the `VaR` margin, in parts per million.
    pub var_ppm: u32,
    /// Column 8, the extreme loss margin, in parts per million.
    pub elm_ppm: u32,
    /// Column 9, NSE's additional margin, in parts per million.
    pub extra_ppm: u32,
}

impl Margin {
    /// `M` under `reading`, in parts per million: columns 7 and 8, plus
    /// column 9 under [`Reading::WithExtra`].
    #[must_use]
    pub fn total_ppm(self, reading: Reading) -> u64 {
        let base = u64::from(self.var_ppm) + u64::from(self.elm_ppm);
        match reading {
            Reading::Day | Reading::Today => base,
            Reading::WithExtra => base + u64::from(self.extra_ppm),
        }
    }
}

/// Why no margin is known for a symbol on a day. Never answered with
/// another day's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarginUnknown {
    /// The day is outside the months the table holds.
    OutsideSpan {
        /// The day asked.
        day: Day,
    },
    /// No batch-1 file of that day was read.
    NoFile {
        /// The day asked.
        day: Day,
    },
    /// The text is not a symbol.
    NotASymbol {
        /// The text.
        text: String,
    },
    /// No file the table was built from lists the symbol.
    NeverListed {
        /// The symbol.
        symbol: Symbol,
        /// The day asked.
        day: Day,
    },
    /// The day's file does not list the symbol.
    NotListed {
        /// The symbol.
        symbol: Symbol,
        /// The day asked.
        day: Day,
    },
}

impl fmt::Display for MarginUnknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideSpan { day } => {
                write!(f, "margin unknown: {day} is outside the months the table holds")
            }
            Self::NoFile { day } => write!(f, "margin unknown: no batch-{BATCH} file of {day} was read"),
            Self::NotASymbol { text } => write!(f, "margin unknown: {text:?} is not a symbol"),
            Self::NeverListed { symbol, day } => {
                write!(f, "margin unknown: {symbol} on {day}: no file read lists it")
            }
            Self::NotListed { symbol, day } => {
                write!(f, "margin unknown: {symbol} on {day}: that day's file does not list it")
            }
        }
    }
}

impl std::error::Error for MarginUnknown {}

/// Which file and columns a leverage reads (SPEC 11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reading {
    /// Columns 7 and 8 of the trade day's file.
    Day,
    /// Columns 7 and 8 of today's file.
    Today,
    /// Columns 7, 8 and 9 of the trade day's file.
    WithExtra,
}

/// An exact positive fraction, reduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: u64,
    den: u64,
}

const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

impl Ratio {
    /// The numerator.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.num
    }

    /// The denominator.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.den
    }
}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let left = u128::from(self.num) * u128::from(other.den);
        let right = u128::from(other.num) * u128::from(self.den);
        left.cmp(&right)
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// `L = min(5, 100 / max(20, M))` (SPEC 11.1), exact and reduced: in parts
/// per million `1,000,000 / max(200,000, M)`, whose floor is the cap.
#[must_use]
pub fn leverage(m: Margin, reading: Reading) -> Ratio {
    let total = m.total_ppm(reading);
    let floor = u64::from(FLOOR_PPM);
    let den = if total > floor { total } else { floor };
    let num = u64::from(WHOLE_PPM);
    let g = gcd(num, den);
    Ratio {
        num: num / g,
        den: den / g,
    }
}

/// The three readings of one stock on one trade day (SPEC 11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Readings {
    /// [`Reading::Day`].
    pub day: Ratio,
    /// [`Reading::Today`].
    pub today: Ratio,
    /// [`Reading::WithExtra`].
    pub with_extra: Ratio,
}

impl Readings {
    /// The smallest: the leverage valid under every reading, which the
    /// quantity uses (SPEC 11.2).
    #[must_use]
    pub fn smallest(self) -> Ratio {
        self.day.min(self.today).min(self.with_extra)
    }
}

/// The dated table, opened.
pub struct MarginTable {
    first: usize,
    months: usize,
    symbols: usize,
    index: HashMap<Symbol, usize>,
    held: Vec<u32>,
    cells: Vec<[u32; 3]>,
    digest: [u8; DIGEST_LEN],
}

impl fmt::Debug for MarginTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (first, last) = self.span();
        write!(
            f,
            "MarginTable {{ span: {first:?}..={last:?}, symbols: {}, digest: {} }}",
            self.symbols,
            self.file_digest_hex()
        )
    }
}

fn cell_of(chunk: &[u8]) -> [u32; 3] {
    [u32_at(chunk, 0), u32_at(chunk, 4), u32_at(chunk, 8)]
}

impl MarginTable {
    /// Opens `<dir>`, the `nse-margin-v1` directory a build wrote: the
    /// dictionary, then each month of its span.
    ///
    /// # Errors
    ///
    /// [`OpenError::Read`] for a file that cannot be read and
    /// [`OpenError::Corrupt`] for one whose bytes fail a check.
    pub fn open(dir: &Path) -> Result<Self, OpenError> {
        let read = |name: &str| {
            let path = dir.join(name);
            fs::read(&path).map_err(|e| OpenError::Read {
                path,
                why: e.to_string(),
            })
        };
        let dictionary = read(DICTIONARY)?;
        let span = read_dictionary(&dictionary)?;
        let months = (span.first..=span.last)
            .map(|at| {
                let (year, month) = month_of(at);
                read(&month_name(year, month))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let months: Vec<&[u8]> = months.iter().map(Vec::as_slice).collect();
        Self::from_bytes(&dictionary, &months)
    }

    /// The table from a dictionary's and its months' bytes, as a build
    /// encodes them ([`encode`]): what [`MarginTable::open`] reads, without
    /// the files.
    ///
    /// # Errors
    ///
    /// [`OpenError::Corrupt`] for bytes that fail a check, a month missing
    /// from the span or one past it.
    pub fn from_bytes(dictionary: &[u8], months: &[&[u8]]) -> Result<Self, OpenError> {
        let span = read_dictionary(dictionary)?;
        if months.len() != span.last - span.first + 1 {
            return Err(corrupt(DICTIONARY, "its span is not the months given"));
        }
        let symbols = span.names.len();
        let bound = brutex_core::blake3::hash(dictionary);
        let mut held = Vec::with_capacity(months.len());
        let mut cells = Vec::with_capacity(months.len() * DAY_SLOTS * symbols);
        for (at, bytes) in (span.first..=span.last).zip(months) {
            let (year, month) = month_of(at);
            held.push(read_month(&month_name(year, month), bytes, at, symbols, &bound)?);
            let body = bytes
                .get(MONTH_HEADER_LEN..bytes.len() - DIGEST_LEN)
                .unwrap_or(&[]);
            cells.extend(body.chunks_exact(CELL_LEN).map(cell_of));
        }
        let index = span.names.iter().enumerate().map(|(i, s)| (*s, i)).collect();
        Ok(Self {
            first: span.first,
            months: months.len(),
            symbols,
            index,
            held,
            cells,
            digest: set_digest(dictionary, months),
        })
    }

    /// The first and last `(year, month)` held.
    #[must_use]
    pub fn span(&self) -> ((u16, u8), (u16, u8)) {
        (month_of(self.first), month_of(self.first + self.months - 1))
    }

    /// Symbols in the dictionary.
    #[must_use]
    pub const fn symbols(&self) -> usize {
        self.symbols
    }

    /// The margin of `symbol` on `day`, from that day's file and no other.
    /// O(1): a month subtraction, a bit, a hash probe and one cell.
    ///
    /// The caller supplies the symbol the share was listed under ON `day`:
    /// across a rename the old and the new symbol are two keys, and neither
    /// answers for the other.
    ///
    /// # Errors
    ///
    /// The [`MarginUnknown`] that names why: never another day's value.
    pub fn margin(&self, symbol: &str, day: Day) -> Result<Margin, MarginUnknown> {
        let month = month_index(day.year(), day.month())
            .checked_sub(self.first)
            .filter(|&month| month < self.months)
            .ok_or(MarginUnknown::OutsideSpan { day })?;
        let slot = usize::from(day.day()) - 1;
        if (self.held.get(month).copied().unwrap_or(0) & (1 << slot)) == 0 {
            return Err(MarginUnknown::NoFile { day });
        }
        let name = Symbol::new(symbol).map_err(|_| MarginUnknown::NotASymbol {
            text: symbol.to_owned(),
        })?;
        let at = *self
            .index
            .get(&name)
            .ok_or(MarginUnknown::NeverListed { symbol: name, day })?;
        let [var_ppm, elm_ppm, extra_ppm] = self
            .cells
            .get((month * DAY_SLOTS + slot) * self.symbols + at)
            .copied()
            .unwrap_or([ABSENT; 3]);
        if var_ppm == ABSENT {
            return Err(MarginUnknown::NotListed { symbol: name, day });
        }
        Ok(Margin {
            day,
            var_ppm,
            elm_ppm,
            extra_ppm,
        })
    }

    /// The three readings of a share on trade day `day`, listed as `symbol`
    /// that day and as `today_symbol` on `today` (SPEC 11.2).
    ///
    /// # Errors
    ///
    /// The [`MarginUnknown`] of either lookup: a missing today's file is
    /// refused for `today`, never answered with the newest file held.
    pub fn readings(
        &self,
        symbol: &str,
        day: Day,
        today_symbol: &str,
        today: Day,
    ) -> Result<Readings, MarginUnknown> {
        let on_day = self.margin(symbol, day)?;
        let on_today = self.margin(today_symbol, today)?;
        Ok(Readings {
            day: leverage(on_day, Reading::Day),
            today: leverage(on_today, Reading::Today),
            with_extra: leverage(on_day, Reading::WithExtra),
        })
    }

    /// BLAKE3 over the dictionary's trailer and every month's, in month
    /// order: the table a run read, for its identity (`leverage_digest`).
    #[must_use]
    pub const fn file_digest(&self) -> [u8; DIGEST_LEN] {
        self.digest
    }

    /// [`Self::file_digest`] in lower-case hex.
    #[must_use]
    pub fn file_digest_hex(&self) -> String {
        hex(&self.digest)
    }
}
