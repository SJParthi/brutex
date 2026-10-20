//! Reference tables as store data, through ONE loader (R3-30; findings
//! DB-B21, DB-B44, SC-CLOSE-15, CMP-05, the store side of DB-G76;
//! failure-matrix rows FM-106, FM-109, FM-263).
//!
//! Seven tables ([`Table::ALL`]): the calendar of record, the index identity
//! rows, the symbol changes, the F&O list, the moved expiries, the dated lot
//! sizes ([`costs::lot::LotTable`], handed its rows here because `costs`
//! names `core` alone) and the risk-free rate series
//! ([`crate::riskfree::Series`], with its documented gaps). Every one has
//! the same receipt, the same checksum rule and the same loader.
//!
//! # Why
//!
//! The index-name table, the dated symbol changes, the F&O list, the moved
//! expiries and the calendar of record were each typed into a crate as a
//! `const` (`brutex_core::alias`, `brutex_core::universe`,
//! [`crate::expiry_alias`], [`crate::calendar`]). A row the exchange added
//! needed a code change and a rebuild, nothing said which copy a running
//! process held, and nothing recorded where a row came from beside the row.
//! Here each table is a FILE in the store, carrying its source, the sha256 of
//! the source bytes when one was read, the day it was read, its status and
//! its decision, and loaded by [`load`] — the one loader every consumer calls
//! at its start. A row added to a table in a store changes the next import
//! that reads that store, with no code change
//! (`a_row_added_to_the_index_table_is_imported_without_a_code_change`).
//!
//! # The bytes on disk
//!
//! ```text
//! <store>/reference/v1/<table>/<generation, 20 digits>.ref
//! ```
//!
//! Text, one item per line:
//!
//! ```text
//! brutex-reference 1
//! table index_identity
//! generation 2
//! source https://www.nseindia.com/api/allIndices
//! source_sha256 <64 hex, or ->
//! fetched 2026-10-07
//! status sourced
//! decision D-3332
//! columns fno|nse_name|gdfl_stem|...
//! row FINNIFTY|NIFTY FINANCIAL SERVICES|NIFTY FIN SERVICE|...
//! rows 1
//! sha256 <64 hex>
//! ```
//!
//! The last line is the SHA-256 of every byte before it, so
//! `sed '$d' <file> | shasum -a 256` repeats the check without this code. A
//! file that fails it, or any line of the grammar, is REFUSED BY NAME
//! ([`Refusal`]: the table, the file, the generation and the reason) and the
//! table is not served from it; nothing falls back to an older generation or
//! to the compiled table in silence. Fields are separated by `|`, which no
//! symbol, index name, circular or URL the tables hold contains; `-` is an
//! absent field.
//!
//! # Append-only
//!
//! [`write`] writes generation `n + 1` to a staging name of its own
//! ([`store::staging::create`]), syncs it and HARD-LINKS it to its final name.
//! A hard link refuses an existing name, so two writers that both saw
//! generation `n` cannot both land: the second is refused
//! ([`WriteError::Raced`]). No generation is ever rewritten (`CLAUDE.md` §3
//! rule 8); every earlier one stays on disk to compare against.
//!
//! # A table with no file
//!
//! Until a store holds a generation of a table, [`load`] serves the table this
//! build compiled, rendered through the same grammar and parsed by the same
//! code as a store file ([`seed_draft`]), and LABELLED: [`Slot::Compiled`]
//! carries the path it looked at. That is the state of every store the day
//! this lands; `seed` installs generation 1 of every absent table from those
//! same drafts. The risk-free rate is the one table this build compiles no
//! copy of ([`Table::compiled`]): until the operator's series lands as a
//! generation ([`rate_draft`]) it is [`RefusalKind::Absent`], by name.
//!
//! # What this does NOT change
//!
//! The swept surface. `CLAUDE.md` §1 names `core::universe::FNO_UNDERLYINGS`
//! as the F&O list and nothing else names it; the [`FnoTable`] answers what is
//! EXPECTED and ASKED on a day, and [`FnoTable::drift_from_swept`] names every
//! difference from the compiled list rather than acting on it. An index row
//! never makes an index sweepable: [`brutex_core::instrument::InstrumentKey`]
//! decides that, and a loaded row is stored, not swept.
//!
//! # Cost
//!
//! [`load`]: per table, one listing of its directory (O(generations), a count
//! that grows only when a table is edited, named for `docs/06-limits.md`), one
//! read of the newest generation (at most [`MAX_FILE_BYTES`]) and one build,
//! O(rows). Measured by `the_loader_reads_every_table_inside_its_budget`
//! (p50, p99 and max at n = 100, at the shipped rows and at ten times them).
//! Every lookup a consumer makes after that is O(1): a probe of a map built at
//! load ([`IndexTable::by_stem`], [`Lineage::from_old`], [`ExpiryTable::alias_of`],
//! [`FnoTable::member_on`]) or one index into a day vector
//! ([`CalendarTable::kind_of`]).
//!
//! # Memory a process keeps
//!
//! The index rows and the symbol changes are handed to
//! [`crate::gdfl_cm::CmScope`] as `&'static` rows, the shape its refusals
//! (`brutex_core::alias::Stale`) already carry. Each distinct table file is
//! interned ONCE per process, keyed by its SHA-256: loading the same
//! generation again reuses it, and memory grows only when a new generation of
//! one of those two tables is loaded — a few kilobytes per edit. Named for
//! `docs/06-limits.md`.

use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Write as _};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use brutex_core::alias::{self, Citation, IndexRow, Stale, Status as RowStatus, SymbolChange, Ymd};
use sha2::{Digest as _, Sha256};

use crate::calendar::{DayKind, MAX_WINDOWS, Session, Window};
use crate::expiry_alias::AliasKind;
use crate::session::Day;

/// Why a store with no `risk_free_rate` generation serves no rate.
pub const NO_COMPILED_RATES: &str = "the store holds no generation of the risk-free rate table \
     and this build compiles none: the RBI 91-day T-bill series is the operator's \
     (docs/00-charter.md section 4h) and lands as a generation from rate_draft; until then every \
     greek refuses by this name, and no rate is invented";

/// The directory under the store root.
pub const DIR: &str = "reference";

/// The file version this build writes and reads.
pub const VERSION: u16 = 1;

/// The first word of every reference file.
const MAGIC: &str = "brutex-reference";

/// The file name suffix of a generation.
pub const SUFFIX: &str = ".ref";

/// The largest reference file a reader opens; a bigger one is refused.
pub const MAX_FILE_BYTES: u64 = 8 * 1_024 * 1_024;

/// The most rows one table may hold.
pub const MAX_ROWS: usize = 200_000;

/// The longest field accepted, in bytes.
pub const MAX_FIELD_BYTES: usize = 2_048;

/// The most membership spans one symbol may hold in the F&O table, so a
/// membership probe is a hash and at most this many comparisons.
pub const MAX_SPANS_PER_SYMBOL: usize = 16;

/// The most generations one table directory may hold before the loader
/// refuses to list it rather than walk it.
pub const MAX_GENERATIONS: usize = 1_000_000;

/// One reference table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Table {
    /// The calendar of record: its span, every holiday and every session
    /// that is not a regular weekday session.
    Calendar,
    /// Every name an index goes by, its key per feed, its span per feed and
    /// the reason for each edge (DB-B21).
    IndexIdentity,
    /// The dated symbol changes NSE published.
    SymbolChanges,
    /// The F&O list and when each name was on it.
    FnoList,
    /// The option expiries a vendor re-dated, or a closed stated day settled
    /// elsewhere ([`crate::expiry_alias`]).
    ExpiryMoves,
    /// The dated lot history per underlying, every row with its source,
    /// the digest of the bytes read and the day ([`costs::lot::LotTable`];
    /// the store side of DB-G76).
    LotSizes,
    /// The RBI 91-day T-bill series greeks discount at, with its documented
    /// gaps ([`crate::riskfree::Series`]; DB-B44).
    RiskFreeRate,
}

impl Table {
    /// Every table, in load order.
    pub const ALL: [Self; 7] = [
        Self::Calendar,
        Self::IndexIdentity,
        Self::SymbolChanges,
        Self::FnoList,
        Self::ExpiryMoves,
        Self::LotSizes,
        Self::RiskFreeRate,
    ];

    /// The table's name on disk and on the wire.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::IndexIdentity => "index_identity",
            Self::SymbolChanges => "symbol_changes",
            Self::FnoList => "fno_list",
            Self::ExpiryMoves => "expiry_moves",
            Self::LotSizes => "lot_sizes",
            Self::RiskFreeRate => "risk_free_rate",
        }
    }

    /// Whether this build compiles a copy of the table that a store with no
    /// generation of it is served from, LABELLED [`Slot::Compiled`]. Only the
    /// risk-free rate has none: its series is the operator's
    /// (`docs/00-charter.md` §4h), so it is [`RefusalKind::Absent`] by name
    /// until a generation lands.
    #[must_use]
    pub const fn compiled(self) -> bool {
        !matches!(self, Self::RiskFreeRate)
    }

    /// The table a word names; `None` for any other text.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.word() == word)
    }

    /// The columns of the table's rows, in order.
    #[must_use]
    pub const fn columns(self) -> &'static [&'static str] {
        match self {
            Self::Calendar => &["kind", "day", "windows", "name", "source"],
            Self::IndexIdentity => &[
                "fno",
                "nse_name",
                "gdfl_stem",
                "kite",
                "key_zerodha",
                "key_gdfl",
                "reference_only",
                "zerodha_first",
                "zerodha_last",
                "gdfl_first",
                "gdfl_last",
                "edges",
                "status",
                "source",
            ],
            Self::SymbolChanges => &["old", "new", "effective", "status", "source"],
            Self::FnoList => &["symbol", "entry", "exit", "source"],
            Self::ExpiryMoves => &[
                "underlying",
                "stated",
                "settled",
                "via",
                "first_day",
                "kind",
                "contracts",
                "source",
            ],
            Self::LotSizes => &[
                "underlying",
                "effective",
                "units",
                "standing",
                "source_sha256",
                "read",
                "source",
            ],
            Self::RiskFreeRate => &[
                "auction_date",
                "settlement_date",
                "released",
                "cut_off_price",
                "implicit_yield_pct",
                "source_url",
                "full_result_url",
                "announcement_url",
            ],
        }
    }

    /// The directory the table's generations live in under `store`.
    #[must_use]
    pub fn dir(self, store: &Path) -> PathBuf {
        store
            .join(DIR)
            .join(format!("v{VERSION}"))
            .join(self.word())
    }

    /// The final name of `generation` under `store`.
    #[must_use]
    pub fn path(self, store: &Path, generation: u64) -> PathBuf {
        self.dir(store).join(format!("{generation:020}{SUFFIX}"))
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// What a whole file rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Standing {
    /// Read from a primary source (a circular, NSE's own file or API).
    Sourced,
    /// Measured from the data (a vendor's files, the store); may label, and
    /// is applied only where the compiled table it replaces was measured too.
    Measured,
    /// Listed so a reader sees it; no lookup applies it.
    Unverified,
}

impl Standing {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Sourced => "sourced",
            Self::Measured => "measured",
            Self::Unverified => "unverified",
        }
    }

    /// The standing a word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [Self::Sourced, Self::Measured, Self::Unverified]
            .into_iter()
            .find(|standing| standing.word() == word)
    }

    /// Whether a lookup may apply a row of this standing.
    #[must_use]
    pub const fn applies(self) -> bool {
        matches!(self, Self::Sourced | Self::Measured)
    }
}

/// The receipt every table file opens with: where its rows came from, when,
/// and what holds them together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// The table.
    pub table: Table,
    /// The generation, as its file name and its own line say.
    pub generation: u64,
    /// Where the rows were read: a URL, a circular, or a measurement.
    pub source: String,
    /// The sha256 of the source bytes as read, when one was recorded.
    pub source_sha256: Option<String>,
    /// The day the source was read.
    pub fetched: Day,
    /// What the file rests on.
    pub standing: Standing,
    /// The decision that owns the table.
    pub decision: String,
    /// How many rows the file holds.
    pub rows: usize,
    /// The SHA-256 of every byte before the checksum line, lower-case hex.
    pub sha256: String,
}

/// A table to write: the receipt's fields and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The table.
    pub table: Table,
    /// Where the rows were read.
    pub source: String,
    /// The sha256 of the source bytes, when one was recorded.
    pub source_sha256: Option<String>,
    /// The day the source was read.
    pub fetched: Day,
    /// What the file rests on.
    pub standing: Standing,
    /// The decision that owns the table.
    pub decision: String,
    /// The rows, each with exactly the table's columns.
    pub rows: Vec<Vec<String>>,
}

/// Why a table file was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefusalKind {
    /// The file or its directory could not be read.
    Read,
    /// The file is larger than [`MAX_FILE_BYTES`].
    TooLarge,
    /// The bytes are not UTF-8 text.
    NotText,
    /// The first line is not `brutex-reference <version>`.
    Magic,
    /// The version is not one this build reads.
    Version,
    /// The file names another table than its directory.
    WrongTable,
    /// The generation line disagrees with the file name.
    Generation,
    /// A receipt line is missing, out of order or malformed.
    Header,
    /// The columns line is not the table's columns.
    Columns,
    /// A row is malformed.
    Row,
    /// Two rows claim one key.
    Duplicate,
    /// The `rows` line disagrees with the rows the file holds.
    RowCount,
    /// The SHA-256 line disagrees with the bytes before it.
    Checksum,
    /// More rows than [`MAX_ROWS`], or more generations than
    /// [`MAX_GENERATIONS`].
    TooMany,
    /// The store holds no generation of a table this build compiles no copy
    /// of: nothing is served, and a consumer refuses by this name.
    Absent,
}

impl RefusalKind {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::TooLarge => "too_large",
            Self::NotText => "not_text",
            Self::Magic => "magic",
            Self::Version => "version",
            Self::WrongTable => "wrong_table",
            Self::Generation => "generation",
            Self::Header => "header",
            Self::Columns => "columns",
            Self::Row => "row",
            Self::Duplicate => "duplicate",
            Self::RowCount => "row_count",
            Self::Checksum => "checksum",
            Self::TooMany => "too_many",
            Self::Absent => "absent",
        }
    }
}

/// A table file refused, BY NAME: the table, the file, the generation, the
/// kind and what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The table.
    pub table: Table,
    /// The file (or directory) refused.
    pub path: PathBuf,
    /// The generation the file name carries, when it carries one.
    pub generation: Option<u64>,
    /// Why.
    pub kind: RefusalKind,
    /// What was found, in words.
    pub detail: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "reference table {} refused {}",
            self.table,
            self.kind.word()
        )?;
        if let Some(generation) = self.generation {
            write!(f, " (generation {generation})")?;
        }
        write!(f, " at {}: {}", self.path.display(), self.detail)
    }
}

impl std::error::Error for Refusal {}

/// A parsed file: its receipt and its rows, each with its line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The receipt.
    pub receipt: Receipt,
    /// The rows, `(line, fields)`.
    pub rows: Vec<(usize, Vec<String>)>,
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _infallible = write!(out, "{byte:02x}");
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn is_hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A field is text with no `|`, no line break and at most
/// [`MAX_FIELD_BYTES`] bytes.
fn field_ok(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_FIELD_BYTES
        && !text.contains(['|', '\n', '\r'])
        && text.trim() == text
}

/// A receipt value is one line of text, not empty, at most
/// [`MAX_FIELD_BYTES`].
fn value_ok(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_FIELD_BYTES && !text.contains(['\n', '\r'])
}

/// Renders `draft` as generation `generation`, checksum included.
///
/// # Errors
///
/// A receipt value or a field that the grammar cannot carry, or a row whose
/// field count is not the table's, named with its row number.
pub fn render(draft: &Draft, generation: u64) -> Result<String, String> {
    let columns = draft.table.columns();
    for (name, value) in [("source", &draft.source), ("decision", &draft.decision)] {
        if !value_ok(value) {
            return Err(format!("{} {name} is not one line of text", draft.table));
        }
    }
    if let Some(sha) = &draft.source_sha256
        && !is_hex64(sha)
    {
        return Err(format!(
            "{} source_sha256 is not 64 lower-case hex",
            draft.table
        ));
    }
    if draft.rows.len() > MAX_ROWS {
        return Err(format!(
            "{} has {} rows, more than {MAX_ROWS}",
            draft.table,
            draft.rows.len()
        ));
    }
    let mut out = String::new();
    let _ = writeln!(out, "{MAGIC} {VERSION}");
    let _ = writeln!(out, "table {}", draft.table);
    let _ = writeln!(out, "generation {generation}");
    let _ = writeln!(out, "source {}", draft.source);
    let _ = writeln!(
        out,
        "source_sha256 {}",
        draft.source_sha256.as_deref().unwrap_or("-")
    );
    let _ = writeln!(out, "fetched {}", draft.fetched);
    let _ = writeln!(out, "status {}", draft.standing.word());
    let _ = writeln!(out, "decision {}", draft.decision);
    let _ = writeln!(out, "columns {}", columns.join("|"));
    for (at, row) in draft.rows.iter().enumerate() {
        if row.len() != columns.len() {
            return Err(format!(
                "{} row {} has {} fields, the table has {}",
                draft.table,
                at + 1,
                row.len(),
                columns.len()
            ));
        }
        if let Some(bad) = row.iter().find(|field| !field_ok(field)) {
            return Err(format!(
                "{} row {} field {bad:?} is empty, padded, too long or holds | or a line break",
                draft.table,
                at + 1
            ));
        }
        let _ = writeln!(out, "row {}", row.join("|"));
    }
    let _ = writeln!(out, "rows {}", draft.rows.len());
    let sum = sha256_hex(out.as_bytes());
    let _ = writeln!(out, "sha256 {sum}");
    Ok(out)
}

/// Strict `YYYY-MM-DD`.
#[must_use]
pub fn parse_day(text: &str) -> Option<Day> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = text.get(range)?;
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let year = u16::try_from(number(0..4)?).ok()?;
    let month = u8::try_from(number(5..7)?).ok()?;
    let day = u8::try_from(number(8..10)?).ok()?;
    Day::new(year, month, day).ok()
}

fn day_field(text: &str) -> Result<Option<Day>, String> {
    if text == "-" {
        return Ok(None);
    }
    parse_day(text)
        .map(Some)
        .ok_or_else(|| format!("{text:?} is not a YYYY-MM-DD day"))
}

fn required_day(text: &str, column: &str) -> Result<Day, String> {
    day_field(text)?.ok_or_else(|| format!("{column} is required"))
}

fn opt(text: &str) -> Option<&str> {
    (text != "-").then_some(text)
}

/// Parses the bytes of `table`'s generation `generation` read from `path`.
///
/// # Errors
///
/// Every [`RefusalKind`] but [`RefusalKind::Read`], named.
#[allow(
    clippy::too_many_lines,
    reason = "one grammar read top to bottom, line by line; split, it would hide the order"
)]
pub fn parse(
    table: Table,
    generation: Option<u64>,
    path: &Path,
    bytes: &[u8],
) -> Result<Parsed, Refusal> {
    let refuse = |kind: RefusalKind, detail: String| Refusal {
        table,
        path: path.to_path_buf(),
        generation,
        kind,
        detail,
    };
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(refuse(
            RefusalKind::TooLarge,
            format!("{} bytes, more than {MAX_FILE_BYTES}", bytes.len()),
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|why| refuse(RefusalKind::NotText, format!("not UTF-8: {why}")))?;
    // THE CHECKSUM FIRST: a file whose bytes are not the bytes written says
    // nothing about its grammar worth reporting.
    let body_end = text
        .trim_end_matches('\n')
        .rfind('\n')
        .map_or(0, |at| at + 1);
    let (body, last) = text.split_at(body_end);
    let Some(claimed) = last
        .strip_prefix("sha256 ")
        .and_then(|rest| rest.strip_suffix('\n'))
    else {
        return Err(refuse(
            RefusalKind::Checksum,
            "the last line is not `sha256 <hex>` followed by a line break: the file is \
             truncated or torn"
                .to_owned(),
        ));
    };
    let actual = sha256_hex(body.as_bytes());
    if claimed != actual {
        return Err(refuse(
            RefusalKind::Checksum,
            format!("the file says sha256 {claimed}; its bytes hash to {actual}"),
        ));
    }
    let mut lines = body.lines().enumerate().map(|(at, line)| (at + 1, line));
    let mut header = |name: &str| -> Result<(usize, String), Refusal> {
        match lines.next() {
            Some((line, text)) => text
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .filter(|value| value_ok(value))
                .map(|value| (line, value.to_owned()))
                .ok_or_else(|| {
                    refuse(
                        RefusalKind::Header,
                        format!("line {line} is not `{name} <value>`"),
                    )
                }),
            None => Err(refuse(
                RefusalKind::Header,
                format!("the file ends before its `{name}` line"),
            )),
        }
    };
    let (_, version) = header(MAGIC).map_err(|mut why| {
        why.kind = RefusalKind::Magic;
        why
    })?;
    if version != VERSION.to_string() {
        return Err(refuse(
            RefusalKind::Version,
            format!("version {version}; this build reads {VERSION}"),
        ));
    }
    let (_, named) = header("table")?;
    if named != table.word() {
        return Err(refuse(
            RefusalKind::WrongTable,
            format!("the file is table {named:?}, under {table}'s directory"),
        ));
    }
    let (line, said) = header("generation")?;
    let said: u64 = said.parse().map_err(|_| {
        refuse(
            RefusalKind::Header,
            format!("line {line}: {said:?} is not a generation"),
        )
    })?;
    if generation.is_some_and(|named| named != said) {
        return Err(refuse(
            RefusalKind::Generation,
            format!("the file says generation {said}; its name says otherwise"),
        ));
    }
    let (_, source) = header("source")?;
    let (line, source_sha256) = header("source_sha256")?;
    let source_sha256 = match source_sha256.as_str() {
        "-" => None,
        sha if is_hex64(sha) => Some(sha.to_owned()),
        other => {
            return Err(refuse(
                RefusalKind::Header,
                format!("line {line}: {other:?} is neither `-` nor 64 lower-case hex"),
            ));
        }
    };
    let (line, fetched) = header("fetched")?;
    let fetched = parse_day(&fetched).ok_or_else(|| {
        refuse(
            RefusalKind::Header,
            format!("line {line}: {fetched:?} is not a YYYY-MM-DD day"),
        )
    })?;
    let (line, standing) = header("status")?;
    let standing = Standing::parse(&standing).ok_or_else(|| {
        refuse(
            RefusalKind::Header,
            format!("line {line}: {standing:?} is not sourced, measured or unverified"),
        )
    })?;
    let (_, decision) = header("decision")?;
    let (line, columns) = header("columns")?;
    if columns != table.columns().join("|") {
        return Err(refuse(
            RefusalKind::Columns,
            format!(
                "line {line}: columns {columns:?}; table {table} has {:?}",
                table.columns().join("|")
            ),
        ));
    }
    let width = table.columns().len();
    let mut rows = Vec::new();
    let mut count_line = None;
    for (line, text) in lines.by_ref() {
        if let Some(fields) = text.strip_prefix("row ") {
            if rows.len() >= MAX_ROWS {
                return Err(refuse(
                    RefusalKind::TooMany,
                    format!("more than {MAX_ROWS} rows"),
                ));
            }
            let fields: Vec<String> = fields.split('|').map(str::to_owned).collect();
            if fields.len() != width || !fields.iter().all(|field| field_ok(field)) {
                return Err(refuse(
                    RefusalKind::Row,
                    format!(
                        "line {line}: {} fields where {table} has {width}, or a field is empty or \
                         padded",
                        fields.len()
                    ),
                ));
            }
            rows.push((line, fields));
        } else if let Some(count) = text.strip_prefix("rows ") {
            count_line = Some((line, count.to_owned()));
            break;
        } else {
            return Err(refuse(
                RefusalKind::Row,
                format!("line {line} is neither `row ...` nor `rows <n>`"),
            ));
        }
    }
    let Some((line, count)) = count_line else {
        return Err(refuse(
            RefusalKind::RowCount,
            "no `rows <n>` line".to_owned(),
        ));
    };
    if count.parse::<usize>().ok() != Some(rows.len()) {
        return Err(refuse(
            RefusalKind::RowCount,
            format!(
                "line {line} says {count:?} rows; the file holds {}",
                rows.len()
            ),
        ));
    }
    if let Some((line, _)) = lines.next() {
        return Err(refuse(
            RefusalKind::Row,
            format!("line {line} follows the `rows` line"),
        ));
    }
    Ok(Parsed {
        receipt: Receipt {
            table,
            generation: said,
            source,
            source_sha256,
            fetched,
            standing,
            decision,
            rows: rows.len(),
            sha256: actual,
        },
        rows,
    })
}

// ─────────────────────────────── the calendar ───────────────────────────────

/// The calendar of record, loaded: O(1) per day.
///
/// The rule the rows are read against is NSE's weekly schedule: Monday to
/// Friday is a regular session (`[09:15, 15:29]`), Saturday and Sunday are
/// closed, unless a `holiday` row closes a weekday or a `session` row records
/// another session (a Muhurat, a drill, a weekend session, an outage day) or
/// a session whose length is not recorded (`unmeasured`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarTable {
    first: i64,
    last: i64,
    kinds: Vec<DayKind>,
    /// Every row's day with its name and source, for a reader that asks why.
    notes: HashMap<i64, (String, String)>,
    /// The sessions of unrecorded length, in day order.
    unsized_days: Vec<i64>,
}

/// Where a calendar's record ends, seen from a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Horizon {
    /// The last day the record answers.
    pub last_day: i64,
    /// Days from the asking day to `last_day`; negative once ended.
    pub days_left: i64,
    /// The asking day is past the record.
    pub ended: bool,
}

/// How many days before a table's record ends its horizon alert fires:
/// **7** (R3-30's goal: "an alert fires 7 days before the calendar or the
/// rate table goes stale"). The tile that shows it is R3-25's.
pub const HORIZON_ALERT_DAYS: i64 = 7;

impl Horizon {
    /// The horizon of a record whose last answered day is `last_day`, seen
    /// from `today`. Two operations.
    #[must_use]
    pub const fn of(last_day: i64, today: i64) -> Self {
        let days_left = last_day.saturating_sub(today);
        Self {
            last_day,
            days_left,
            ended: days_left < 0,
        }
    }

    /// Whether the alert fires: the record ends within
    /// [`HORIZON_ALERT_DAYS`] of the asking day, or has ended.
    #[must_use]
    pub const fn alert(&self) -> bool {
        self.days_left <= HORIZON_ALERT_DAYS
    }
}

/// A day whose verdict a newer calendar changes (FM-263).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retraction {
    /// The day.
    pub day: i64,
    /// What the older calendar said.
    pub was: DayKind,
    /// What the newer one says.
    pub now: DayKind,
    /// Whether the asker holds bars on the day.
    pub held: bool,
}

fn hhmm(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

fn parse_hhmm(text: &str) -> Option<u16> {
    let (h, m) = text.split_once(':')?;
    if h.len() != 2 || m.len() != 2 || !(h.bytes().chain(m.bytes())).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// `HH:MM-HH:MM[,HH:MM-HH:MM]`, inclusive minutes, as the calendar's windows.
fn render_windows(session: Session) -> String {
    session
        .windows
        .iter()
        .take(usize::from(session.count).min(MAX_WINDOWS))
        .map(|w| format!("{}-{}", hhmm(w.from), hhmm(w.to)))
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_windows(text: &str) -> Option<Session> {
    let mut windows = [Window { from: 0, to: 0 }; MAX_WINDOWS];
    let mut count = 0_usize;
    let mut floor: Option<u16> = None;
    for part in text.split(',') {
        let (from, to) = part.split_once('-')?;
        let window = Window {
            from: parse_hhmm(from)?,
            to: parse_hhmm(to)?,
        };
        if window.from > window.to || floor.is_some_and(|floor| window.from <= floor) {
            return None;
        }
        floor = Some(window.to);
        *windows.get_mut(count)? = window;
        count += 1;
    }
    Some(Session {
        windows,
        count: u8::try_from(count).ok()?,
    })
}

const fn weekday_rule(day: i64) -> DayKind {
    // 1970-01-01, day 0, was a Thursday: Monday..Friday are 0..=4 here.
    if (day + 3).rem_euclid(7) < 5 {
        DayKind::Open(Session::full())
    } else {
        DayKind::Closed
    }
}

impl CalendarTable {
    #[allow(
        clippy::too_many_lines,
        reason = "one pass over the rows and one over the span, each arm a row kind"
    )]
    fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let refuse = |kind, line: usize, detail: String| Refusal {
            table: Table::Calendar,
            path: path.to_path_buf(),
            generation: Some(parsed.receipt.generation),
            kind,
            detail: format!("line {line}: {detail}"),
        };
        let mut first = None;
        let mut last = None;
        for (line, row) in &parsed.rows {
            let (Some(kind), Some(day)) = (row.first(), row.get(1)) else {
                continue;
            };
            let day = i64::from(
                required_day(day, "day")
                    .map_err(|why| refuse(RefusalKind::Row, *line, why))?
                    .days_from_epoch(),
            );
            match kind.as_str() {
                "first" if first.replace(day).is_some() => {
                    return Err(refuse(
                        RefusalKind::Duplicate,
                        *line,
                        "a second `first` row".into(),
                    ));
                }
                "last" if last.replace(day).is_some() => {
                    return Err(refuse(
                        RefusalKind::Duplicate,
                        *line,
                        "a second `last` row".into(),
                    ));
                }
                _ => {}
            }
        }
        let (Some(first), Some(last)) = (first, last) else {
            return Err(refuse(
                RefusalKind::Row,
                0,
                "a calendar needs one `first` and one `last` row".into(),
            ));
        };
        let span = usize::try_from(last - first + 1)
            .ok()
            .filter(|span| (1..=MAX_ROWS).contains(span))
            .ok_or_else(|| {
                refuse(
                    RefusalKind::Row,
                    0,
                    format!("span {first}..={last} is empty or longer than {MAX_ROWS} days"),
                )
            })?;
        let mut kinds: Vec<DayKind> = (first..=last).map(weekday_rule).collect();
        debug_assert_eq!(kinds.len(), span);
        let mut notes = HashMap::with_capacity(parsed.rows.len());
        let mut unsized_days = Vec::new();
        for (line, row) in &parsed.rows {
            let [kind, day, windows, name, source] = row.as_slice() else {
                return Err(refuse(RefusalKind::Row, *line, "wrong field count".into()));
            };
            let day = i64::from(
                required_day(day, "day")
                    .map_err(|why| refuse(RefusalKind::Row, *line, why))?
                    .days_from_epoch(),
            );
            let answer = match (kind.as_str(), windows.as_str()) {
                ("first" | "last", "-") => continue,
                ("holiday", "-") => DayKind::Closed,
                ("session", "unmeasured") => DayKind::OpenLengthUnmeasured,
                ("session", windows) => DayKind::Open(parse_windows(windows).ok_or_else(|| {
                    refuse(
                        RefusalKind::Row,
                        *line,
                        format!(
                            "windows {windows:?} are not ascending HH:MM-HH:MM, at most \
                             {MAX_WINDOWS}"
                        ),
                    )
                })?),
                (kind, windows) => {
                    return Err(refuse(
                        RefusalKind::Row,
                        *line,
                        format!(
                            "kind {kind:?} with windows {windows:?}: a row is first, last or \
                             holiday with `-`, or session with windows or `unmeasured`"
                        ),
                    ));
                }
            };
            let slot = usize::try_from(day - first)
                .ok()
                .and_then(|at| kinds.get_mut(at))
                .ok_or_else(|| {
                    refuse(
                        RefusalKind::Row,
                        *line,
                        format!("day {day} lies outside the calendar's own span"),
                    )
                })?;
            if notes.insert(day, (name.clone(), source.clone())).is_some() {
                return Err(refuse(
                    RefusalKind::Duplicate,
                    *line,
                    "a second row for one day".into(),
                ));
            }
            if answer == DayKind::OpenLengthUnmeasured {
                unsized_days.push(day);
            }
            *slot = answer;
        }
        unsized_days.sort_unstable();
        Ok(Self {
            first,
            last,
            kinds,
            notes,
            unsized_days,
        })
    }

    /// What `day` was. One compare, one subtract, one index: O(1).
    #[must_use]
    pub fn kind_of(&self, day: i64) -> DayKind {
        if !(self.first..=self.last).contains(&day) {
            return DayKind::Unmeasured;
        }
        usize::try_from(day - self.first)
            .ok()
            .and_then(|at| self.kinds.get(at))
            .copied()
            .unwrap_or(DayKind::Unmeasured)
    }

    /// The first day the record answers.
    #[must_use]
    pub const fn first_day(&self) -> i64 {
        self.first
    }

    /// The last day the record answers.
    #[must_use]
    pub const fn last_day(&self) -> i64 {
        self.last
    }

    /// The name and the source of `day`'s row, when it has one. One probe.
    #[must_use]
    pub fn note(&self, day: i64) -> Option<(&str, &str)> {
        self.notes
            .get(&day)
            .map(|(name, source)| (name.as_str(), source.as_str()))
    }

    /// Where the record ends, seen from `today`. Two operations.
    #[must_use]
    pub const fn horizon(&self, today: i64) -> Horizon {
        Horizon::of(self.last, today)
    }

    /// Every session the record holds whose length is not recorded, from
    /// `today` on, ONE item per day with its name and source: what an
    /// operator must supply before the day imports (FM-106). A day stays one
    /// item however many runs meet it. O(sessions of unrecorded length).
    #[must_use]
    pub fn unsized_from(&self, today: i64) -> Vec<(i64, &str, &str)> {
        self.unsized_days
            .iter()
            .filter(|day| **day >= today)
            .map(|day| {
                let (name, source) = self.note(*day).unwrap_or(("", ""));
                (*day, name, source)
            })
            .collect()
    }

    /// Every day of `self`'s span whose verdict `newer` changes, with
    /// whether `held` says the asker holds bars on it (FM-263): what must be
    /// named before the newer calendar is applied. O(days of the span) — the
    /// span is the record's own length, named for `docs/06-limits.md`.
    #[must_use]
    pub fn retractions(&self, newer: &Self, held: impl Fn(i64) -> bool) -> Vec<Retraction> {
        (self.first..=self.last)
            .filter_map(|day| {
                let (was, now) = (self.kind_of(day), newer.kind_of(day));
                (was != now).then(|| Retraction {
                    day,
                    was,
                    now,
                    held: held(day),
                })
            })
            .collect()
    }
}

// ───────────────────────────── the index rows ─────────────────────────────

/// One index under every name it goes by, with its span on each feed and why
/// each edge is where it is (DB-B21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexIdentity {
    /// The symbol its derivatives carry, when it has any.
    pub fno: Option<String>,
    /// NSE's index name.
    pub nse_name: String,
    /// NSE's `indexSymbol`, the GDFL file stem.
    pub gdfl_stem: String,
    /// The Kite tradingsymbol, when Zerodha lists it.
    pub kite: Option<String>,
    /// The key the store files Zerodha's series under, when Zerodha has it.
    pub key_zerodha: Option<String>,
    /// The key the store files GDFL's series under, when GDFL has it.
    pub key_gdfl: Option<String>,
    /// Stored and stamped, never swept.
    pub reference_only: bool,
    /// Zerodha's first and last day, when measured.
    pub zerodha_span: (Option<Day>, Option<Day>),
    /// GDFL's first and last day, when measured.
    pub gdfl_span: (Option<Day>, Option<Day>),
    /// Why each edge is where it is (`-` for none recorded).
    pub edges: Option<String>,
    /// What the row rests on.
    pub standing: Standing,
    /// Where it was read.
    pub source: String,
}

/// The index table, loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexTable {
    rows: Vec<IndexIdentity>,
    by_stem: HashMap<String, usize>,
    /// The applied rows with a GDFL key, as `brutex_core::alias` rows, in
    /// table order: what [`crate::gdfl_cm::CmScope`] files.
    applied: &'static [IndexRow],
}

impl IndexTable {
    fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let refuse = |kind, line: usize, detail: String| Refusal {
            table: Table::IndexIdentity,
            path: path.to_path_buf(),
            generation: Some(parsed.receipt.generation),
            kind,
            detail: format!("line {line}: {detail}"),
        };
        let mut rows = Vec::with_capacity(parsed.rows.len());
        let mut by_stem = HashMap::with_capacity(parsed.rows.len());
        let mut keys = BTreeSet::new();
        for (line, row) in &parsed.rows {
            let [
                fno,
                nse_name,
                gdfl_stem,
                kite,
                key_zerodha,
                key_gdfl,
                reference_only,
                zf,
                zl,
                gf,
                gl,
                edges,
                status,
                source,
            ] = row.as_slice()
            else {
                return Err(refuse(RefusalKind::Row, *line, "wrong field count".into()));
            };
            let bad = |why: String| refuse(RefusalKind::Row, *line, why);
            let reference_only = match reference_only.as_str() {
                "yes" => true,
                "no" => false,
                other => return Err(bad(format!("reference_only {other:?} is not yes or no"))),
            };
            let standing = Standing::parse(status)
                .ok_or_else(|| bad(format!("status {status:?} is not a standing")))?;
            for key in [key_zerodha, key_gdfl].into_iter().filter_map(|k| opt(k)) {
                brutex_core::instrument::InstrumentKey::index(
                    brutex_core::instrument::Exchange::Nse,
                    key,
                )
                .map_err(|why| bad(format!("store key {key:?} is not an index key: {why}")))?;
            }
            let identity = IndexIdentity {
                fno: opt(fno).map(str::to_owned),
                nse_name: nse_name.clone(),
                gdfl_stem: gdfl_stem.clone(),
                kite: opt(kite).map(str::to_owned),
                key_zerodha: opt(key_zerodha).map(str::to_owned),
                key_gdfl: opt(key_gdfl).map(str::to_owned),
                reference_only,
                zerodha_span: (day_field(zf).map_err(&bad)?, day_field(zl).map_err(&bad)?),
                gdfl_span: (day_field(gf).map_err(&bad)?, day_field(gl).map_err(&bad)?),
                edges: opt(edges).map(str::to_owned),
                standing,
                source: source.clone(),
            };
            for (first, last) in [identity.zerodha_span, identity.gdfl_span] {
                if let (Some(first), Some(last)) = (first, last)
                    && first.days_from_epoch() > last.days_from_epoch()
                {
                    return Err(bad(format!("span {first}..{last} runs backwards")));
                }
            }
            if by_stem.insert(gdfl_stem.clone(), rows.len()).is_some() {
                return Err(refuse(
                    RefusalKind::Duplicate,
                    *line,
                    format!("GDFL stem {gdfl_stem:?} is on two rows"),
                ));
            }
            for (feed, key) in [
                ("zerodha", &identity.key_zerodha),
                ("gdfl", &identity.key_gdfl),
            ] {
                if let Some(key) = key
                    && !keys.insert((feed, key.clone()))
                {
                    return Err(refuse(
                        RefusalKind::Duplicate,
                        *line,
                        format!("{feed} key {key:?} is on two rows"),
                    ));
                }
            }
            rows.push(identity);
        }
        let applied = intern_index_rows(&parsed.receipt, &rows);
        Ok(Self {
            rows,
            by_stem,
            applied,
        })
    }

    /// Every row, in table order.
    #[must_use]
    pub fn rows(&self) -> &[IndexIdentity] {
        &self.rows
    }

    /// The row whose GDFL stem is `stem`. One probe.
    #[must_use]
    pub fn by_stem(&self, stem: &str) -> Option<&IndexIdentity> {
        self.by_stem.get(stem).and_then(|at| self.rows.get(*at))
    }

    /// The applied rows with a GDFL key, as `brutex_core::alias` rows.
    #[must_use]
    pub const fn applied(&self) -> &'static [IndexRow] {
        self.applied
    }
}

/// The interned tables of this process, by the SHA-256 of their file.
struct Interned {
    indices: HashMap<String, &'static [IndexRow]>,
    lineages: HashMap<String, &'static Lineage>,
}

fn interned() -> &'static Mutex<Interned> {
    static INTERNED: OnceLock<Mutex<Interned>> = OnceLock::new();
    INTERNED.get_or_init(|| {
        Mutex::new(Interned {
            indices: HashMap::new(),
            lineages: HashMap::new(),
        })
    })
}

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

/// The citations a row's own `source` column names, when it is wholly in the
/// shape [`cited`] writes: parts joined by `; `, each `<url> read
/// <YYYY-MM-DD>` and optionally ` sha256 <64 hex>`; `None` for any other
/// text. A URL may itself hold `; ` or ` read ` (the compiled ones do), so a
/// part ends only where a dated `read` is followed by the end or by `; `.
/// This is what keeps a seeded row's provenance through the store: its own
/// URL, read day and checksum, not the file's (R3-30 repair 1).
fn own_citations(source: &str) -> Option<Vec<Citation>> {
    fn day_shaped(day: &str) -> bool {
        day.len() == 10
            && day.bytes().enumerate().all(|(at, byte)| {
                if at == 4 || at == 7 {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            })
    }
    // After ` read `: the day, the optional checksum, and what follows the
    // part (`Some("")` at the end), or `None` when this is not a part's end.
    fn dated(tail: &str) -> Option<(&str, Option<&str>, &str)> {
        let read = tail.get(..10).filter(|day| day_shaped(day))?;
        let mut after = tail.get(10..)?;
        let mut sha = None;
        if let Some(hex) = after
            .strip_prefix(" sha256 ")
            .and_then(|rest| rest.get(..64))
            .filter(|hex| hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            sha = Some(hex);
            after = after.get(8 + 64..)?;
        }
        if after.is_empty() {
            Some((read, sha, ""))
        } else {
            after.strip_prefix("; ").map(|next| (read, sha, next))
        }
    }
    let mut rest = source;
    let mut own = Vec::new();
    while !rest.is_empty() {
        let mut from = 0;
        let (url, read, sha, next) = loop {
            let at = from + rest.get(from..)?.find(" read ")?;
            if let Some((read, sha, next)) = rest.get(at + 6..).and_then(dated) {
                break (rest.get(..at)?, read, sha, next);
            }
            from = at + 1;
        };
        if url.is_empty() {
            return None;
        }
        own.push(Citation {
            url: leak(url),
            read: leak(read),
            sha256: sha.map(leak),
            says: leak(source),
        });
        rest = next;
    }
    (!own.is_empty()).then_some(own)
}

fn citation_of(receipt: &Receipt, source: &str) -> &'static [Citation] {
    if let Some(own) = own_citations(source) {
        return Box::leak(own.into_boxed_slice());
    }
    // A source column in any other words (an operator's own row) cites the
    // file that holds it, with the row's words as what it says.
    let citation = Citation {
        url: leak(&receipt.source),
        read: leak(&receipt.fetched.to_string()),
        sha256: receipt.source_sha256.as_deref().map(leak),
        says: leak(source),
    };
    Box::leak(vec![citation].into_boxed_slice())
}

fn row_status(receipt: &Receipt, standing: Standing, source: &str) -> RowStatus {
    if standing.applies() && receipt.standing.applies() {
        RowStatus::Sourced(citation_of(receipt, source))
    } else {
        RowStatus::Unverified(leak(source))
    }
}

fn intern_index_rows(receipt: &Receipt, rows: &[IndexIdentity]) -> &'static [IndexRow] {
    let build = || -> &'static [IndexRow] {
        let applied: Vec<IndexRow> = rows
            .iter()
            .filter_map(|row| {
                let key = row.key_gdfl.as_deref()?;
                Some(IndexRow {
                    fno: row.fno.as_deref().map(leak),
                    nse_name: leak(&row.nse_name),
                    index_symbol: leak(&row.gdfl_stem),
                    kite: leak(row.kite.as_deref().unwrap_or("")),
                    store_key: leak(key),
                    reference_only: row.reference_only,
                    status: row_status(receipt, row.standing, &row.source),
                })
            })
            .collect();
        Box::leak(applied.into_boxed_slice())
    };
    let Ok(mut held) = interned().lock() else {
        // A poisoned lock means another thread panicked mid-insert; the map
        // is still a map, but this build is not worth trusting with it:
        // build a private copy rather than share a half-written one.
        return build();
    };
    held.indices
        .entry(receipt.sha256.clone())
        .or_insert_with(build)
}

// ──────────────────────────── the symbol changes ────────────────────────────

/// The dated symbol changes, loaded, with O(1) probes by the old and by the
/// new symbol. The same answers as `brutex_core::alias::Lineage` over the
/// same rows (`the_seeded_lineage_answers_exactly_what_the_compiled_one_does`).
#[derive(Debug, PartialEq, Eq)]
pub struct Lineage {
    rows: &'static [SymbolChange],
    by_old: HashMap<&'static str, usize>,
    by_new: HashMap<&'static str, usize>,
}

impl Lineage {
    fn of(rows: &'static [SymbolChange]) -> Result<Self, String> {
        let mut by_old = HashMap::with_capacity(rows.len());
        let mut by_new = HashMap::with_capacity(rows.len());
        for (at, row) in rows.iter().enumerate() {
            if by_old.insert(row.old, at).is_some() {
                return Err(format!("{:?} changes away twice", row.old));
            }
            if by_new.insert(row.new, at).is_some() {
                return Err(format!("{:?} is reached twice", row.new));
            }
        }
        let lineage = Self {
            rows,
            by_old,
            by_new,
        };
        for row in rows {
            if lineage.forward(row.old).any(|hop| hop.new == row.old) {
                return Err(format!("the chain from {:?} returns to it", row.old));
            }
        }
        Ok(lineage)
    }

    /// The compiled lineage of `brutex_core::alias`, built once, or the
    /// reason the compiled rows do not form one. Never an empty lineage in
    /// its place: an empty lineage would file every stem under its own name
    /// and refuse none, which is a fallback that hides the fault (R3-30
    /// repair 1). `the_shipped_lineage_has_no_cycle_and_no_duplicate` holds
    /// the compiled rows.
    ///
    /// # Errors
    ///
    /// The compiled rows' cycle or duplicate, in words, on every call.
    pub fn compiled() -> Result<&'static Self, &'static str> {
        static COMPILED: OnceLock<Result<Lineage, String>> = OnceLock::new();
        COMPILED
            .get_or_init(|| Self::of(alias::LINEAGE.rows()))
            .as_ref()
            .map_err(String::as_str)
    }

    /// Every row, applied or not, in table order.
    #[must_use]
    pub const fn rows(&self) -> &'static [SymbolChange] {
        self.rows
    }

    /// The applied change away from `old`. One probe.
    #[must_use]
    pub fn from_old(&self, old: &str) -> Option<&'static SymbolChange> {
        let rows = self.rows;
        self.by_old
            .get(old)
            .and_then(|at| rows.get(*at))
            .filter(|row| row.status.applies())
    }

    /// The applied change that produced `new`. One probe.
    #[must_use]
    pub fn to_new(&self, new: &str) -> Option<&'static SymbolChange> {
        let rows = self.rows;
        self.by_new
            .get(new)
            .and_then(|at| rows.get(*at))
            .filter(|row| row.status.applies())
    }

    /// The applied changes from `symbol` forward, oldest first; at most one
    /// hop per row, so a cycle cannot loop.
    pub fn forward<'a>(
        &'a self,
        symbol: &'a str,
    ) -> impl Iterator<Item = &'static SymbolChange> + 'a {
        let mut at: &str = symbol;
        (0..self.rows.len()).map_while(move |_| {
            let hop = self.from_old(at)?;
            at = hop.new;
            Some(hop)
        })
    }

    /// The applied changes that led to `symbol`, newest first.
    pub fn backward<'a>(
        &'a self,
        symbol: &'a str,
    ) -> impl Iterator<Item = &'static SymbolChange> + 'a {
        let mut at: &str = symbol;
        (0..self.rows.len()).map_while(move |_| {
            let hop = self.to_new(at)?;
            at = hop.old;
            Some(hop)
        })
    }

    /// The newest symbol `symbol`'s applied changes reach, or `symbol`.
    #[must_use]
    pub fn current<'a>(&'a self, symbol: &'a str) -> &'a str {
        self.forward(symbol).last().map_or(symbol, |hop| hop.new)
    }

    /// The key a vendor file named `stem` and dated `day` is filed under:
    /// `brutex_core::alias::Lineage::filed_under`'s rule over these rows.
    ///
    /// # Errors
    ///
    /// [`Stale::BeforeChange`] for a new symbol before its change,
    /// [`Stale::AfterChange`] for an old symbol on or after it.
    pub fn filed_under<'a>(&'a self, stem: &'a str, day: Ymd) -> Result<&'a str, Stale> {
        if let Some(made) = self.to_new(stem)
            && day < made.effective
        {
            return Err(Stale::BeforeChange(*made));
        }
        if let Some(gone) = self.from_old(stem)
            && day >= gone.effective
        {
            return Err(Stale::AfterChange(*gone));
        }
        Ok(self.current(stem))
    }
}

fn build_lineage(parsed: &Parsed, path: &Path) -> Result<&'static Lineage, Refusal> {
    let refuse = |kind, line: usize, detail: String| Refusal {
        table: Table::SymbolChanges,
        path: path.to_path_buf(),
        generation: Some(parsed.receipt.generation),
        kind,
        detail: if line == 0 {
            detail
        } else {
            format!("line {line}: {detail}")
        },
    };
    let mut changes = Vec::with_capacity(parsed.rows.len());
    for (line, row) in &parsed.rows {
        let [old, new, effective, status, source] = row.as_slice() else {
            return Err(refuse(RefusalKind::Row, *line, "wrong field count".into()));
        };
        let effective = required_day(effective, "effective")
            .map_err(|why| refuse(RefusalKind::Row, *line, why))?;
        let standing = Standing::parse(status).ok_or_else(|| {
            refuse(
                RefusalKind::Row,
                *line,
                format!("status {status:?} is not a standing"),
            )
        })?;
        for symbol in [old, new] {
            brutex_core::instrument::InstrumentKey::cash(
                brutex_core::instrument::Exchange::Nse,
                symbol,
            )
            .map_err(|why| {
                refuse(
                    RefusalKind::Row,
                    *line,
                    format!("{symbol:?} is not a share symbol: {why}"),
                )
            })?;
        }
        if old == new {
            return Err(refuse(
                RefusalKind::Row,
                *line,
                format!("{old:?} changes to itself"),
            ));
        }
        changes.push((
            old.clone(),
            new.clone(),
            effective,
            standing,
            source.clone(),
        ));
    }
    let Ok(mut held) = interned().lock() else {
        return Err(refuse(
            RefusalKind::Read,
            0,
            "the process's table interner is poisoned by an earlier panic".into(),
        ));
    };
    if let Some(lineage) = held.lineages.get(&parsed.receipt.sha256) {
        return Ok(lineage);
    }
    let rows: Vec<SymbolChange> = changes
        .iter()
        .map(|(old, new, effective, standing, source)| SymbolChange {
            old: leak(old),
            new: leak(new),
            effective: Ymd::new(effective.year(), effective.month(), effective.day()),
            status: row_status(&parsed.receipt, *standing, source),
        })
        .collect();
    let rows: &'static [SymbolChange] = Box::leak(rows.into_boxed_slice());
    let lineage = Lineage::of(rows).map_err(|why| refuse(RefusalKind::Duplicate, 0, why))?;
    let lineage: &'static Lineage = Box::leak(Box::new(lineage));
    held.lineages.insert(parsed.receipt.sha256.clone(), lineage);
    Ok(lineage)
}

// ─────────────────────────────── the F&O list ───────────────────────────────

/// One span of a name on the F&O list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoSpan {
    /// The first day on the list; `None` when not measured.
    pub entry: Option<Day>,
    /// The last day on the list; `None` while still on it.
    pub exit: Option<Day>,
    /// Where it was read.
    pub source: String,
}

/// Whether a name was on the F&O list on a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Membership {
    /// On the list.
    Member,
    /// Not on the list.
    NotMember,
    /// The table does not know: the span's entry is not measured and the day
    /// is before the day the list was read.
    Unmeasured,
}

/// The F&O list, loaded: a probe by name and a bounded walk of that name's
/// spans (at most [`MAX_SPANS_PER_SYMBOL`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoTable {
    listed_on: Day,
    symbols: Vec<String>,
    spans: HashMap<String, Vec<FnoSpan>>,
}

impl FnoTable {
    fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let refuse = |kind, line: usize, detail: String| Refusal {
            table: Table::FnoList,
            path: path.to_path_buf(),
            generation: Some(parsed.receipt.generation),
            kind,
            detail: format!("line {line}: {detail}"),
        };
        let mut symbols = Vec::new();
        let mut spans: HashMap<String, Vec<FnoSpan>> = HashMap::new();
        for (line, row) in &parsed.rows {
            let [symbol, entry, exit, source] = row.as_slice() else {
                return Err(refuse(RefusalKind::Row, *line, "wrong field count".into()));
            };
            let bad = |why: String| refuse(RefusalKind::Row, *line, why);
            let span = FnoSpan {
                entry: day_field(entry).map_err(&bad)?,
                exit: day_field(exit).map_err(&bad)?,
                source: source.clone(),
            };
            if let (Some(entry), Some(exit)) = (span.entry, span.exit)
                && entry.days_from_epoch() > exit.days_from_epoch()
            {
                return Err(bad(format!("span {entry}..{exit} runs backwards")));
            }
            let held = spans.entry(symbol.clone()).or_default();
            if held.is_empty() {
                symbols.push(symbol.clone());
            }
            let start = |s: &FnoSpan| s.entry.map_or(0, Day::days_from_epoch);
            let end = |s: &FnoSpan| s.exit.map_or(u32::MAX, Day::days_from_epoch);
            if held
                .iter()
                .any(|other| start(other) <= end(&span) && start(&span) <= end(other))
            {
                return Err(refuse(
                    RefusalKind::Duplicate,
                    *line,
                    format!("{symbol:?} has two spans that overlap"),
                ));
            }
            if held.len() >= MAX_SPANS_PER_SYMBOL {
                return Err(refuse(
                    RefusalKind::TooMany,
                    *line,
                    format!("{symbol:?} has more than {MAX_SPANS_PER_SYMBOL} spans"),
                ));
            }
            held.push(span);
        }
        Ok(Self {
            listed_on: parsed.receipt.fetched,
            symbols,
            spans,
        })
    }

    /// Whether `symbol` was on the list on `day`: one probe and at most
    /// [`MAX_SPANS_PER_SYMBOL`] comparisons.
    #[must_use]
    pub fn member_on(&self, symbol: &str, day: Day) -> Membership {
        let Some(spans) = self.spans.get(symbol) else {
            return Membership::NotMember;
        };
        let at = day.days_from_epoch();
        let mut answer = Membership::NotMember;
        for span in spans {
            if span.exit.is_some_and(|exit| at > exit.days_from_epoch()) {
                continue;
            }
            match span.entry {
                Some(entry) if at >= entry.days_from_epoch() => return Membership::Member,
                Some(_) => {}
                None if at >= self.listed_on.days_from_epoch() => return Membership::Member,
                None => answer = Membership::Unmeasured,
            }
        }
        answer
    }

    /// Every name the table lists, in table order.
    #[must_use]
    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    /// The names on the list from the day it was read, in table order.
    #[must_use]
    pub fn current(&self) -> Vec<&str> {
        self.symbols
            .iter()
            .filter(|symbol| self.member_on(symbol, self.listed_on) == Membership::Member)
            .map(String::as_str)
            .collect()
    }

    /// The names this table lists today and the compiled swept list does
    /// not, and the reverse: NAMED, never applied. The swept surface is
    /// `core::universe::FNO_UNDERLYINGS` and nothing else (`CLAUDE.md` §1).
    #[must_use]
    pub fn drift_from_swept(&self) -> (Vec<String>, Vec<&'static str>) {
        let current: BTreeSet<&str> = self.current().into_iter().collect();
        let swept: BTreeSet<&'static str> =
            brutex_core::universe::FNO_UNDERLYINGS.into_iter().collect();
        (
            current
                .difference(&swept.iter().copied().collect())
                .map(|s| (*s).to_owned())
                .collect(),
            swept
                .iter()
                .filter(|symbol| !current.contains(**symbol))
                .copied()
                .collect(),
        )
    }
}

// ───────────────────────────── the moved expiries ─────────────────────────────

/// One moved expiry, as [`crate::expiry_alias::ExpiryAlias`] holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryMove {
    /// The underlying, or `None` for every underlying.
    pub underlying: Option<String>,
    /// The expiry the vendor's text states.
    pub stated: Day,
    /// The day the contract settled.
    pub settled: Day,
    /// The intermediate text of a chain.
    pub via: Option<Day>,
    /// The rename day (re-dated) or the first listed day (closed day).
    pub first_day: Day,
    /// Why.
    pub kind: AliasKind,
    /// How many names carried the stated text.
    pub contracts: u32,
    /// Where it was read.
    pub source: String,
}

/// The moved expiries, loaded: one probe for an underlying's own row and one
/// for the every-underlying row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiryTable {
    rows: Vec<ExpiryMove>,
    by_key: HashMap<(Option<String>, u32), usize>,
}

impl ExpiryTable {
    fn build(parsed: &Parsed, path: &Path) -> Result<Self, Refusal> {
        let refuse = |kind, line: usize, detail: String| Refusal {
            table: Table::ExpiryMoves,
            path: path.to_path_buf(),
            generation: Some(parsed.receipt.generation),
            kind,
            detail: format!("line {line}: {detail}"),
        };
        let mut rows = Vec::with_capacity(parsed.rows.len());
        let mut by_key = HashMap::with_capacity(parsed.rows.len());
        for (line, row) in &parsed.rows {
            let [
                underlying,
                stated,
                settled,
                via,
                first_day,
                kind,
                contracts,
                source,
            ] = row.as_slice()
            else {
                return Err(refuse(RefusalKind::Row, *line, "wrong field count".into()));
            };
            let bad = |why: String| refuse(RefusalKind::Row, *line, why);
            let moved = ExpiryMove {
                underlying: (underlying != "*").then(|| underlying.clone()),
                stated: required_day(stated, "stated").map_err(&bad)?,
                settled: required_day(settled, "settled").map_err(&bad)?,
                via: day_field(via).map_err(&bad)?,
                first_day: required_day(first_day, "first_day").map_err(&bad)?,
                kind: AliasKind::parse(kind)
                    .ok_or_else(|| bad(format!("kind {kind:?} is not a move kind")))?,
                contracts: contracts
                    .parse()
                    .map_err(|_| bad(format!("contracts {contracts:?} is not a count")))?,
                source: source.clone(),
            };
            let key = (moved.underlying.clone(), moved.stated.days_from_epoch());
            if by_key.insert(key, rows.len()).is_some() {
                return Err(refuse(
                    RefusalKind::Duplicate,
                    *line,
                    format!("{underlying} {stated} is on two rows"),
                ));
            }
            rows.push(moved);
        }
        Ok(Self { rows, by_key })
    }

    /// Every row, in table order.
    #[must_use]
    pub fn rows(&self) -> &[ExpiryMove] {
        &self.rows
    }

    /// `underlying`'s own row for `stated`, else the every-underlying row:
    /// [`crate::expiry_alias::alias_of`]'s rule, two probes.
    #[must_use]
    pub fn alias_of(&self, underlying: &str, stated: Day) -> Option<&ExpiryMove> {
        let at = stated.days_from_epoch();
        self.by_key
            .get(&(Some(underlying.to_owned()), at))
            .or_else(|| self.by_key.get(&(None, at)))
            .and_then(|at| self.rows.get(*at))
    }
}

// ──────────────────────────── the lot sizes ────────────────────────────

/// The lot table, loaded: `costs::lot::LotTable` built from the store's
/// rows. `costs` names `core` alone and never `pull` (`CLAUDE.md` §5), so
/// this loader parses the rows and HANDS them to it.
///
/// A row is applied only when it AND the file are `sourced`: a file whose
/// receipt is `measured` or `unverified` lists every row and applies none
/// (`CLAUDE.md` §3 rule 1). An underlying or a day with no sourced row is a
/// named [`costs::lot::LotAbsence`], never a guessed or inferred lot: the
/// vendor's `ltq` GCD and its `inferred` label are composed downstream
/// (R3-10-OPT), and the `gdfl.lots` ledger is not read here.
fn build_lots(parsed: &Parsed, path: &Path) -> Result<costs::lot::LotTable, Refusal> {
    use costs::lot::{LotRow, LotStanding, LotTable};
    let refuse = |kind, detail: String| Refusal {
        table: Table::LotSizes,
        path: path.to_path_buf(),
        generation: Some(parsed.receipt.generation),
        kind,
        detail,
    };
    let file_sourced = parsed.receipt.standing == Standing::Sourced;
    let mut rows = Vec::with_capacity(parsed.rows.len());
    for (line, row) in &parsed.rows {
        let bad = |why: String| refuse(RefusalKind::Row, format!("line {line}: {why}"));
        let [underlying, effective, units, standing, sha, read, source] = row.as_slice() else {
            return Err(bad("wrong field count".into()));
        };
        let effective = required_day(effective, "effective")
            .map_err(&bad)
            .and_then(|day| {
                crate::pricing::trade_day_of(day)
                    .ok_or_else(|| bad(format!("effective {day} is outside the trade-day window")))
            })?;
        let read = day_field(read).map_err(&bad)?;
        let read = match read {
            None => None,
            Some(day) => Some(
                crate::pricing::trade_day_of(day)
                    .ok_or_else(|| bad(format!("read {day} is outside the trade-day window")))?,
            ),
        };
        let units: u32 = units
            .parse()
            .map_err(|_| bad(format!("units {units:?} is not a count")))?;
        let standing = LotStanding::parse(standing).ok_or_else(|| {
            bad(format!(
                "standing {standing:?} is not sourced or unverified"
            ))
        })?;
        rows.push(LotRow {
            underlying: underlying.clone(),
            effective,
            units,
            standing: if file_sourced {
                standing
            } else {
                LotStanding::Unverified
            },
            source: source.clone(),
            source_sha256: opt(sha).map(str::to_owned),
            read,
        });
    }
    LotTable::from_rows(rows).map_err(|why| {
        let line = match &why {
            costs::lot::LotTableError::TooManyRows { .. } => None,
            costs::lot::LotTableError::Text { row, .. }
            | costs::lot::LotTableError::ZeroLot { row }
            | costs::lot::LotTableError::SourcedWithoutReceipt { row }
            | costs::lot::LotTableError::NotAscending { row }
            | costs::lot::LotTableError::TooManyForUnderlying { row } => row
                .checked_sub(1)
                .and_then(|at| parsed.rows.get(at))
                .map(|(line, _)| *line),
        };
        let kind = RefusalKind::Row;
        refuse(
            kind,
            line.map_or_else(|| why.to_string(), |line| format!("line {line}: {why}")),
        )
    })
}

/// The lot rows of a store table as a [`Draft`]: what `costs::lot`
/// compiled for NIFTY and BANKNIFTY, unchanged in day, units and citation
/// ([`costs::lot::compiled_rows`]).
fn lot_seed() -> Result<Draft, String> {
    let rows = costs::lot::compiled_rows()
        .into_iter()
        .map(|row| {
            vec![
                field(&row.underlying),
                row.effective.to_string(),
                row.units.to_string(),
                row.standing.word().to_owned(),
                row.source_sha256
                    .as_deref()
                    .map_or_else(|| "-".to_owned(), field),
                row.read
                    .map_or_else(|| "-".to_owned(), |day| day.to_string()),
                field(&row.source),
            ]
        })
        .collect();
    Ok(Draft {
        table: Table::LotSizes,
        source: "costs::lot::LOT_SIZES (NIFTY and BANKNIFTY), each row sourced exactly where \
                 docs/00-charter.md records its circular (NSE/FAOP/64625, NSE/FAOP/70616), \
                 every other row unverified and not applied"
            .to_owned(),
        source_sha256: None,
        fetched: ymd_day((2026, 10, 10))?,
        standing: Standing::Sourced,
        decision: "D-3357".to_owned(),
        rows,
    })
}

// ────────────────────────── the risk-free rate ──────────────────────────

/// The rate table, loaded: the store's rows read back through
/// [`crate::riskfree::Series::parse`], the ONE parser of the series, so a
/// row the operator's file would refuse is refused here by the same rule.
fn build_rates(parsed: &Parsed, path: &Path) -> Result<crate::riskfree::Series, Refusal> {
    let mut text = String::from(crate::riskfree::HEADER);
    for (_, row) in &parsed.rows {
        text.push('\n');
        let fields: Vec<&str> = row.iter().map(|field| opt(field).unwrap_or("")).collect();
        text.push_str(&fields.join("\t"));
    }
    text.push('\n');
    crate::riskfree::Series::parse(&text).map_err(|why| {
        let (line, detail) = match &why {
            crate::riskfree::SeriesError::Row { line, fault } => (
                line.checked_sub(2)
                    .and_then(|at| parsed.rows.get(at))
                    .map(|(line, _)| *line),
                format!("{fault:?}"),
            ),
            other => (None, format!("{other:?}")),
        };
        Refusal {
            table: Table::RiskFreeRate,
            path: path.to_path_buf(),
            generation: Some(parsed.receipt.generation),
            kind: RefusalKind::Row,
            detail: line.map_or_else(
                || format!("the series refused: {detail}"),
                |line| format!("line {line}: the series refused the row: {detail}"),
            ),
        }
    })
}

/// The operator's RBI 91-day T-bill table (`riskfree::HEADER` and its rows,
/// gap rows included) as a [`Draft`] of the store's `risk_free_rate` table:
/// the bytes are checked by [`crate::riskfree::Series::parse`] first, and
/// their sha256 is the draft's `source_sha256`. This build compiles no copy
/// of the series (`docs/00-charter.md` §4h keeps it the operator's), so
/// until a generation lands the table is [`RefusalKind::Absent`] by name.
///
/// # Errors
///
/// The series' own refusal, or a field the grammar cannot carry.
pub fn rate_draft(tsv: &str, source: &str, fetched: Day) -> Result<Draft, String> {
    crate::riskfree::Series::parse(tsv).map_err(|why| format!("{why}"))?;
    let mut rows = Vec::new();
    for (offset, line) in tsv.lines().skip(1).enumerate() {
        let fields: Vec<String> = line
            .split('\t')
            .map(|text| {
                if text.is_empty() {
                    "-".to_owned()
                } else {
                    text.to_owned()
                }
            })
            .collect();
        if let Some(bad) = fields.iter().find(|text| !field_ok(text)) {
            return Err(format!(
                "line {}: {bad:?} cannot be carried by the reference grammar",
                offset + 2
            ));
        }
        rows.push(fields);
    }
    Ok(Draft {
        table: Table::RiskFreeRate,
        source: source.to_owned(),
        source_sha256: Some(sha256_hex(tsv.as_bytes())),
        fetched,
        standing: Standing::Sourced,
        decision: "D-2810".to_owned(),
        rows,
    })
}

// ─────────────────────────────── the loader ───────────────────────────────

/// Where one table came from, and the table.
#[derive(Debug, Clone)]
pub enum Slot<T> {
    /// A store generation, checksum verified.
    Store {
        /// Its receipt.
        receipt: Receipt,
        /// Its file.
        path: PathBuf,
        /// The table.
        table: Arc<T>,
    },
    /// No generation in the store: the table this build compiled, through
    /// the same grammar, labelled with the directory that was empty.
    Compiled {
        /// The compiled table's receipt.
        receipt: Receipt,
        /// The directory that holds no generation.
        looked_in: PathBuf,
        /// The table.
        table: Arc<T>,
    },
    /// The newest generation was refused: nothing is served from this
    /// table, and a consumer that needs it refuses by this name.
    Refused(Refusal),
}

impl<T> Slot<T> {
    /// The table, or the refusal that keeps it from being served.
    ///
    /// # Errors
    ///
    /// [`Slot::Refused`]'s refusal.
    pub fn table(&self) -> Result<&Arc<T>, &Refusal> {
        match self {
            Self::Store { table, .. } | Self::Compiled { table, .. } => Ok(table),
            Self::Refused(why) => Err(why),
        }
    }

    /// The receipt of what is served, when something is.
    #[must_use]
    pub const fn receipt(&self) -> Option<&Receipt> {
        match self {
            Self::Store { receipt, .. } | Self::Compiled { receipt, .. } => Some(receipt),
            Self::Refused(_) => None,
        }
    }

    /// `store`, `compiled` or `refused`.
    #[must_use]
    pub const fn origin(&self) -> &'static str {
        match self {
            Self::Store { .. } => "store",
            Self::Compiled { .. } => "compiled",
            Self::Refused(_) => "refused",
        }
    }

    /// The file served, the directory looked in, or the file refused.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Store { path, .. } => path,
            Self::Compiled { looked_in, .. } => looked_in,
            Self::Refused(why) => &why.path,
        }
    }
}

/// Every reference table a process serves, each from the store or labelled
/// compiled or refused.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The calendar of record.
    pub calendar: Slot<CalendarTable>,
    /// The index rows.
    pub indices: Slot<IndexTable>,
    /// The symbol changes.
    pub renames: Slot<&'static Lineage>,
    /// The F&O list.
    pub fno: Slot<FnoTable>,
    /// The moved expiries.
    pub expiry: Slot<ExpiryTable>,
    /// The dated lot sizes.
    pub lots: Slot<costs::lot::LotTable>,
    /// The risk-free rate series.
    pub rates: Slot<crate::riskfree::Series>,
    /// Every table whose directory held names that are not generations, in
    /// load order, each told as a `warn` event: never served, never ignored
    /// in silence.
    pub strays: Vec<(Table, Strays)>,
}

/// One slot as a reader sees it: the table, where it came from (`store`,
/// `compiled` or `refused`), the file or directory, the receipt of what is
/// served and the refusal of what is not.
pub type SlotView<'a> = (
    Table,
    &'static str,
    &'a Path,
    Option<&'a Receipt>,
    Option<&'a Refusal>,
);

impl Reference {
    /// The `(table, origin, path, receipt)` of every slot, in load order.
    #[must_use]
    pub fn slots(&self) -> [SlotView<'_>; 7] {
        fn of<T>(table: Table, slot: &Slot<T>) -> SlotView<'_> {
            (
                table,
                slot.origin(),
                slot.path(),
                slot.receipt(),
                slot.table().err(),
            )
        }
        [
            of(Table::Calendar, &self.calendar),
            of(Table::IndexIdentity, &self.indices),
            of(Table::SymbolChanges, &self.renames),
            of(Table::FnoList, &self.fno),
            of(Table::ExpiryMoves, &self.expiry),
            of(Table::LotSizes, &self.lots),
            of(Table::RiskFreeRate, &self.rates),
        ]
    }

    /// Where the two dated records end, seen from `today` (days since the
    /// epoch): the calendar of record's last day, and the last day the rate
    /// series answers (its last release plus
    /// [`crate::riskfree::MAX_AGE_DAYS`]). A table that is not served
    /// answers its own [`Refusal`], never a quiet `None` (R3-30 repair 1).
    /// O(1).
    pub fn horizons(&self, today: i64) -> [(Table, Result<Horizon, &Refusal>); 2] {
        let calendar = self.calendar.table().map(|table| table.horizon(today));
        let rates = self.rates.table().map(|series| {
            let last = i64::from(series.last_release().ordinal())
                + i64::from(crate::riskfree::MAX_AGE_DAYS);
            Horizon::of(last, today)
        });
        [(Table::Calendar, calendar), (Table::RiskFreeRate, rates)]
    }

    /// Every dated record that needs the operator, seen from `today`: one
    /// whose horizon alert fires ([`HORIZON_ALERT_DAYS`]) or has ended, and
    /// one that is not served at all, each with its sentence. An absent or
    /// refused record is an alert, not a silence. O(1).
    #[must_use]
    pub fn horizon_alerts(&self, today: i64) -> Vec<(Table, String)> {
        self.horizons(today)
            .into_iter()
            .filter_map(|(table, horizon)| match horizon {
                Ok(horizon) if horizon.ended => Some((
                    table,
                    format!("ended {} days ago", horizon.days_left.unsigned_abs()),
                )),
                Ok(horizon) if horizon.alert() => {
                    Some((table, format!("ends in {} days", horizon.days_left)))
                }
                Ok(_) => None,
                Err(why) => Some((table, format!("not served: {why}"))),
            })
            .collect()
    }

    /// Every refusal, in load order.
    #[must_use]
    pub fn refusals(&self) -> Vec<&Refusal> {
        self.slots()
            .into_iter()
            .filter_map(|(_, _, _, _, refused)| refused)
            .collect()
    }

    /// Every table this build compiled, labelled as such: what a process
    /// with no store serves. Built once per process.
    #[must_use]
    pub fn compiled() -> Arc<Self> {
        static COMPILED: OnceLock<Arc<Reference>> = OnceLock::new();
        Arc::clone(COMPILED.get_or_init(|| Arc::new(load(Path::new("")))))
    }
}

/// The names in a table's directory that are not generations. None is ever
/// served, and none is ignored in silence (R3-30 repair 1): [`load`] keeps
/// them in [`Reference::strays`] and tells each table's as one `warn` event,
/// so a misnamed generation is seen beside the table that was served
/// without it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Strays {
    /// Staging files (`.<generation>.<pid>.<serial>.staging`): a write in
    /// flight, or one killed before its hard link.
    pub staging: usize,
    /// Every other name: misnamed, foreign or hand-placed.
    pub other: usize,
    /// The smallest other name, so the event can name one.
    pub first_other: Option<String>,
}

impl Strays {
    /// No stray at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.staging == 0 && self.other == 0
    }

    fn count(&mut self, name: &std::ffi::OsStr) {
        let text = name.to_string_lossy();
        if text.starts_with('.') && text.ends_with(".staging") {
            self.staging += 1;
        } else {
            self.other += 1;
            if self
                .first_other
                .as_deref()
                .is_none_or(|first| *text < *first)
            {
                self.first_other = Some(text.into_owned());
            }
        }
    }
}

/// The newest generation's number and path in `table`'s directory under
/// `store`, the names that are not generations, or `Ok(None)` when the
/// directory holds no generation.
fn newest(store: &Path, table: Table) -> Result<(Option<(u64, PathBuf)>, Strays), Refusal> {
    let dir = table.dir(store);
    let refuse = |kind, detail: String| Refusal {
        table,
        path: dir.clone(),
        generation: None,
        kind,
        detail,
    };
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Ok((None, Strays::default()));
        }
        Err(why) => return Err(refuse(RefusalKind::Read, why.to_string())),
    };
    let mut best: Option<u64> = None;
    let mut strays = Strays::default();
    for (seen, entry) in entries.enumerate() {
        if seen >= MAX_GENERATIONS {
            return Err(refuse(
                RefusalKind::TooMany,
                format!("more than {MAX_GENERATIONS} names"),
            ));
        }
        let entry = entry.map_err(|why| refuse(RefusalKind::Read, why.to_string()))?;
        let name = entry.file_name();
        let generation = name
            .to_str()
            .and_then(|name| name.strip_suffix(SUFFIX))
            .filter(|digits| digits.len() == 20 && digits.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|digits| digits.parse::<u64>().ok());
        match generation {
            Some(generation) => best = best.max(Some(generation)),
            None => strays.count(&name),
        }
    }
    Ok((best.map(|g| (g, table.path(store, g))), strays))
}

fn read_capped(path: &Path) -> Result<Vec<u8>, (RefusalKind, String)> {
    let (file, len) = store::safe_open::open_read_len(path)
        .map_err(|why| (RefusalKind::Read, why.to_string()))?;
    if len > MAX_FILE_BYTES {
        return Err((
            RefusalKind::TooLarge,
            format!("{len} bytes, more than {MAX_FILE_BYTES}"),
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| (RefusalKind::Read, why.to_string()))?;
    Ok(bytes)
}

/// Loads one table: the store's newest generation, else the compiled seed.
fn slot<T>(
    store: &Path,
    table: Table,
    build: impl Fn(&Parsed, &Path) -> Result<T, Refusal>,
) -> (Slot<T>, Strays) {
    let compiled = || -> Result<(Receipt, T), Refusal> {
        let dir = table.dir(store);
        if !table.compiled() {
            return Err(Refusal {
                table,
                path: dir,
                generation: None,
                kind: RefusalKind::Absent,
                detail: NO_COMPILED_RATES.to_owned(),
            });
        }
        let draft = seed_draft(table).map_err(|why| Refusal {
            table,
            path: dir.clone(),
            generation: None,
            kind: RefusalKind::Row,
            detail: format!("the compiled table cannot be rendered: {why}"),
        })?;
        let text = render(&draft, 0).map_err(|why| Refusal {
            table,
            path: dir.clone(),
            generation: None,
            kind: RefusalKind::Row,
            detail: format!("the compiled table cannot be rendered: {why}"),
        })?;
        let parsed = parse(table, Some(0), &dir, text.as_bytes())?;
        let made = build(&parsed, &dir)?;
        Ok((parsed.receipt, made))
    };
    let found = if store.as_os_str().is_empty() {
        Ok((None, Strays::default()))
    } else {
        newest(store, table)
    };
    let (found, strays) = match found {
        Err(why) => return (Slot::Refused(why), Strays::default()),
        Ok(found) => found,
    };
    let slot = match found {
        None => match compiled() {
            Ok((receipt, made)) => Slot::Compiled {
                receipt,
                looked_in: table.dir(store),
                table: Arc::new(made),
            },
            Err(why) => Slot::Refused(why),
        },
        Some((generation, path)) => {
            let loaded = read_capped(&path)
                .map_err(|(kind, detail)| Refusal {
                    table,
                    path: path.clone(),
                    generation: Some(generation),
                    kind,
                    detail,
                })
                .and_then(|bytes| parse(table, Some(generation), &path, &bytes))
                .and_then(|parsed| Ok((build(&parsed, &path)?, parsed.receipt)));
            match loaded {
                Ok((made, receipt)) => Slot::Store {
                    receipt,
                    path,
                    table: Arc::new(made),
                },
                Err(why) => Slot::Refused(why),
            }
        }
    };
    (slot, strays)
}

/// One `warn` event for a table directory that holds names which are not
/// generations, naming the count of each kind and the smallest other name.
fn told_strays(store: &Path, table: Table, strays: &Strays) {
    if strays.is_empty() {
        return;
    }
    let dir = table.dir(store).display().to_string();
    let event = telemetry::Event::warn("pull::reference_data", "names that are not generations")
        .with("table", table.word())
        .with("path", dir.as_str())
        .with("staging", u64::try_from(strays.staging).unwrap_or(u64::MAX))
        .with("other", u64::try_from(strays.other).unwrap_or(u64::MAX))
        .with("first_other", strays.first_other.as_deref().unwrap_or("-"));
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// The level and message a slot is told at. A table this build has no
/// compiled copy of, and the store no generation of yet, is an ABSENCE,
/// said at `warn` with its reason, not a refused file at `error`: every run
/// that loads the tables (an import never reads the rate) would otherwise
/// log an error for a table nobody wrote (R3-30 repair 1). A consumer that
/// needs the table still refuses by this name.
fn told_as<T>(slot: &Slot<T>) -> (telemetry::Level, &'static str) {
    match slot {
        Slot::Refused(why) if why.kind == RefusalKind::Absent && why.generation.is_none() => {
            (telemetry::Level::Warn, "table absent")
        }
        Slot::Refused(_) => (telemetry::Level::Error, "table refused"),
        Slot::Store { .. } | Slot::Compiled { .. } => (telemetry::Level::Info, "table loaded"),
    }
}

fn told<T>(table: Table, slot: &Slot<T>) {
    let path = slot.path().display().to_string();
    let fetched = slot
        .receipt()
        .map(|receipt| receipt.fetched.to_string())
        .unwrap_or_default();
    let (level, message) = told_as(slot);
    let event =
        telemetry::Event::new(level, "pull::reference_data", message).with("table", table.word());
    let event = match slot {
        Slot::Refused(why) => event
            .with("path", path.as_str())
            .with("why", why.kind.word())
            .with("detail", why.detail.as_str()),
        Slot::Store { receipt, .. } | Slot::Compiled { receipt, .. } => event
            .with("origin", slot.origin())
            .with("path", path.as_str())
            .with("generation", receipt.generation)
            .with("rows", u64::try_from(receipt.rows).unwrap_or(u64::MAX))
            .with("source", receipt.source.as_str())
            .with("fetched", fetched.as_str())
            .with("sha256", receipt.sha256.as_str()),
    };
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// THE ONE LOADER. Every table, from `store` (an empty path loads the
/// compiled tables alone), each told as one event with its origin, source
/// and checksum, or its refusal.
#[must_use]
pub fn load(store: &Path) -> Reference {
    let mut strays = Vec::new();
    let calendar = kept(&mut strays, store, Table::Calendar, CalendarTable::build);
    let indices = kept(&mut strays, store, Table::IndexIdentity, IndexTable::build);
    let renames = kept(&mut strays, store, Table::SymbolChanges, build_lineage);
    let fno = kept(&mut strays, store, Table::FnoList, FnoTable::build);
    let expiry = kept(&mut strays, store, Table::ExpiryMoves, ExpiryTable::build);
    let lots = kept(&mut strays, store, Table::LotSizes, build_lots);
    let rates = kept(&mut strays, store, Table::RiskFreeRate, build_rates);
    Reference {
        calendar,
        indices,
        renames,
        fno,
        expiry,
        lots,
        rates,
        strays,
    }
}

/// One table through [`slot`], told as its event, its strays told and kept.
fn kept<T>(
    strays: &mut Vec<(Table, Strays)>,
    store: &Path,
    table: Table,
    build: impl Fn(&Parsed, &Path) -> Result<T, Refusal>,
) -> Slot<T> {
    let (loaded, found) = slot(store, table, build);
    told(table, &loaded);
    told_strays(store, table, &found);
    if !found.is_empty() {
        strays.push((table, found));
    }
    loaded
}

// ─────────────────────────────── the writer ───────────────────────────────

/// A generation that landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The table.
    pub table: Table,
    /// The generation.
    pub generation: u64,
    /// Its final name.
    pub path: PathBuf,
    /// Its checksum.
    pub sha256: String,
    /// `true` when this write landed the generation; `false` when the newest
    /// generation already held exactly these bytes and nothing was written.
    pub landed: bool,
}

/// Why a write did not land.
#[derive(Debug)]
pub enum WriteError {
    /// The draft is not something the grammar can carry.
    Draft(String),
    /// The directory could not be listed or created, or the file written.
    Io(String),
    /// Another writer landed this generation first: nothing of this write
    /// is at a final name.
    Raced(u64),
    /// The newest generation is `u64::MAX`: no generation follows it.
    Exhausted(u64),
    /// The draft does not parse back as its own table.
    Refused(Refusal),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft(why) => write!(f, "draft refused: {why}"),
            Self::Io(why) => write!(f, "write failed: {why}"),
            Self::Raced(generation) => {
                write!(
                    f,
                    "generation {generation} was written by another writer first"
                )
            }
            Self::Exhausted(generation) => write!(
                f,
                "generation {generation} is the last a name can carry; no generation follows it"
            ),
            Self::Refused(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for WriteError {}

/// The per-process serial in a staging name, so two threads of one process
/// writing one generation never share a staging file
/// (`two_writers_of_one_generation_never_both_land`).
static WRITER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes `draft` as the next generation of its table under `store`.
///
/// # Errors
///
/// Every [`WriteError`]; a refused draft writes nothing at a final name.
pub fn write(store: &Path, draft: &Draft) -> Result<Written, WriteError> {
    let table = draft.table;
    let dir = table.dir(store);
    std::fs::create_dir_all(&dir)
        .map_err(|why| WriteError::Io(format!("{}: {why}", dir.display())))?;
    let newest_found = match newest(store, table) {
        Ok((found, _)) => found,
        Err(why) => return Err(WriteError::Refused(why)),
    };
    // IDEMPOTENT (R3-30 repair 1): a draft whose bytes, numbered as the
    // newest generation, ARE that generation lands nothing; re-running a
    // seed or a rate import over the same rows is not a new generation.
    if let Some((held, held_path)) = &newest_found
        && let Ok(rendered) = render(draft, *held)
        && read_capped(held_path).is_ok_and(|bytes| bytes == rendered.as_bytes())
    {
        let parsed = parse(table, Some(*held), held_path, rendered.as_bytes())
            .map_err(WriteError::Refused)?;
        return Ok(Written {
            table,
            generation: *held,
            path: held_path.clone(),
            sha256: parsed.receipt.sha256,
            landed: false,
        });
    }
    let generation = match newest_found {
        None => 1,
        // The last generation a name can carry: refused by name, never
        // reported as a race the writer did not lose.
        Some((held, _)) => held.checked_add(1).ok_or(WriteError::Exhausted(held))?,
    };
    let text = render(draft, generation).map_err(WriteError::Draft)?;
    let path = table.path(store, generation);
    // The draft must read back as its own table before it lands: a table
    // that cannot be served is never published.
    let parsed =
        parse(table, Some(generation), &path, text.as_bytes()).map_err(WriteError::Refused)?;
    let probe: Result<(), Refusal> = match table {
        Table::Calendar => CalendarTable::build(&parsed, &path).map(drop),
        Table::IndexIdentity => IndexTable::build(&parsed, &path).map(drop),
        Table::SymbolChanges => build_lineage(&parsed, &path).map(drop),
        Table::FnoList => FnoTable::build(&parsed, &path).map(drop),
        Table::ExpiryMoves => ExpiryTable::build(&parsed, &path).map(drop),
        Table::LotSizes => build_lots(&parsed, &path).map(drop),
        Table::RiskFreeRate => build_rates(&parsed, &path).map(drop),
    };
    probe.map_err(WriteError::Refused)?;
    // The staging name is this WRITER's own: the process and a per-process
    // serial ([`WRITER`]), so two threads of one process writing one
    // generation never share a staging file.
    let writer = WRITER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = dir.join(format!(
        ".{generation:020}.{}.{writer}.staging",
        std::process::id()
    ));
    let mut staged = store::staging::create(&staging)
        .map_err(|why| WriteError::Io(format!("{}: {why}", staging.display())))?;
    staged
        .file
        .write_all(text.as_bytes())
        .and_then(|()| staged.file.sync_all())
        .map_err(|why| WriteError::Io(format!("{}: {why}", staging.display())))?;
    let linked = std::fs::hard_link(&staging, &path);
    // The staging name is this writer's own: removing it changes no byte of
    // the generation, which now has its final name (or was refused).
    let _left_for_the_next_create = std::fs::remove_file(&staging);
    match linked {
        // DURABLE: the new name is synced in its directory before the write
        // reports it landed, so a power cut after `Ok` cannot lose it.
        Ok(()) => std::fs::File::open(&dir)
            .and_then(|handle| handle.sync_all())
            .map_err(|why| {
                WriteError::Io(format!(
                    "{}: generation {generation} is linked but its directory did not sync: {why}",
                    dir.display()
                ))
            })
            .map(|()| Written {
                table,
                generation,
                path,
                sha256: parsed.receipt.sha256,
                landed: true,
            }),
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(WriteError::Raced(generation))
        }
        Err(why) => Err(WriteError::Io(format!("{}: {why}", path.display()))),
    }
}

/// Installs generation 1 of every table `store` holds no generation of,
/// from [`seed_draft`]: the compiled tables, now as store data. A table
/// already in the store is left as it is. Returns what landed.
///
/// # Errors
///
/// The first [`WriteError`]; tables before it have landed and are named
/// by a re-run, which writes only what is still absent.
pub fn seed(store: &Path) -> Result<Vec<Written>, WriteError> {
    let mut landed = Vec::new();
    for table in Table::ALL.into_iter().filter(|table| table.compiled()) {
        match newest(store, table) {
            Ok((Some(_), _)) => continue,
            Ok((None, _)) => {}
            Err(why) => return Err(WriteError::Refused(why)),
        }
        let draft = seed_draft(table).map_err(WriteError::Draft)?;
        landed.push(write(store, &draft)?);
    }
    Ok(landed)
}

// ─────────────────────────────── the seeds ───────────────────────────────

fn ymd_day(ymd: (u16, u8, u8)) -> Result<Day, String> {
    Day::new(ymd.0, ymd.1, ymd.2).map_err(|why| format!("{ymd:?}: {why}"))
}

fn day_of(epoch: i64) -> Result<Day, String> {
    u32::try_from(epoch)
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .ok_or_else(|| format!("day {epoch} is not a civil day"))
}

fn text_of(day: Option<Day>) -> String {
    day.map_or_else(|| "-".to_owned(), |day| day.to_string())
}

fn cited(status: RowStatus) -> (Standing, String) {
    match status {
        RowStatus::Sourced(citations) => (
            Standing::Sourced,
            // Each citation's sha256 travels with it when the charter
            // records one (R3-30 repair 1: the seed dropped it).
            citations
                .iter()
                .map(|c| {
                    c.sha256.map_or_else(
                        || format!("{} read {}", c.url, c.read),
                        |sha| format!("{} read {} sha256 {sha}", c.url, c.read),
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        RowStatus::Unverified(why) => (Standing::Unverified, why.to_owned()),
    }
}

/// One field: `text` as it is when the grammar can carry it, else `-`
/// so a seed never fails on an empty compiled value.
fn field(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        "-".to_owned()
    } else {
        text.replace(['|', '\n', '\r'], " ")
    }
}

/// The compiled table `table` of this build as a [`Draft`]: what [`load`]
/// serves for a store with no generation, and what [`seed`] installs.
///
/// # Errors
///
/// A compiled value the grammar cannot carry, named.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per table, each the compiled table as rows"
)]
pub fn seed_draft(table: Table) -> Result<Draft, String> {
    match table {
        Table::Calendar => calendar_seed(),
        Table::IndexIdentity => Ok(Draft {
            table,
            source: alias::CHARTER.to_owned(),
            source_sha256: None,
            fetched: ymd_day((2026, 10, 7))?,
            standing: Standing::Sourced,
            decision: alias::DECISION.to_owned(),
            rows: alias::INDEX_ROWS
                .iter()
                .map(|row| {
                    let (standing, source) = cited(row.status);
                    vec![
                        row.fno.map_or_else(|| "-".to_owned(), field),
                        field(row.nse_name),
                        field(row.index_symbol),
                        field(row.kite),
                        field(row.store_key),
                        field(row.store_key),
                        if row.reference_only { "yes" } else { "no" }.to_owned(),
                        "-".to_owned(),
                        "-".to_owned(),
                        "-".to_owned(),
                        "-".to_owned(),
                        "-".to_owned(),
                        standing.word().to_owned(),
                        field(&source),
                    ]
                })
                .collect(),
        }),
        Table::SymbolChanges => Ok(Draft {
            table,
            source: alias::CHARTER.to_owned(),
            source_sha256: None,
            fetched: ymd_day((2026, 10, 7))?,
            standing: Standing::Sourced,
            decision: alias::DECISION.to_owned(),
            rows: alias::LINEAGE
                .rows()
                .iter()
                .map(|row| {
                    let (standing, source) = cited(row.status);
                    vec![
                        field(row.old),
                        field(row.new),
                        row.effective.to_string(),
                        standing.word().to_owned(),
                        field(&source),
                    ]
                })
                .collect(),
        }),
        Table::FnoList => Ok(Draft {
            table,
            source: "core::universe::FNO_UNDERLYINGS, derived from both vendor masters and \
                     recorded in docs/00-charter.md section 4a"
                .to_owned(),
            source_sha256: None,
            fetched: ymd_day((2026, 8, 1))?,
            standing: Standing::Measured,
            decision: "D-0506".to_owned(),
            rows: brutex_core::universe::FNO_UNDERLYINGS
                .iter()
                .map(|symbol| {
                    vec![
                        field(symbol),
                        "-".to_owned(),
                        "-".to_owned(),
                        "on the list the day it was read; entry not measured".to_owned(),
                    ]
                })
                .collect(),
        }),
        Table::LotSizes => lot_seed(),
        Table::RiskFreeRate => Err(NO_COMPILED_RATES.to_owned()),
        Table::ExpiryMoves => Ok(Draft {
            table,
            source: "measured from the GDFL options name census, 2026-10-07, re-measured \
                     2026-10-11 (pull::expiry_alias)"
                .to_owned(),
            source_sha256: None,
            fetched: ymd_day((2026, 10, 11))?,
            standing: Standing::Measured,
            decision: "D-3331".to_owned(),
            rows: crate::expiry_alias::ALIASES
                .iter()
                .map(|row| -> Result<Vec<String>, String> {
                    Ok(vec![
                        row.underlying.map_or_else(|| "*".to_owned(), field),
                        ymd_day(row.stated)?.to_string(),
                        ymd_day(row.settled)?.to_string(),
                        text_of(row.via.map(ymd_day).transpose()?),
                        ymd_day(row.first_day)?.to_string(),
                        row.kind.as_str().to_owned(),
                        row.contracts.to_string(),
                        // Each row says what it rests on (R3-30 repair 1).
                        field(&row.basis.source()),
                    ])
                })
                .collect::<Result<_, _>>()?,
        }),
    }
}

fn calendar_seed() -> Result<Draft, String> {
    use crate::calendar::{
        SOURCED_FIRST_DAY, SOURCED_LAST_DAY, exchange_kind_of, holiday_note, special_note,
        special_session,
    };
    let mut rows = vec![
        vec![
            "first".to_owned(),
            day_of(SOURCED_FIRST_DAY)?.to_string(),
            "-".to_owned(),
            "the calendar of record begins".to_owned(),
            "pull::calendar::SOURCED_FIRST_DAY".to_owned(),
        ],
        vec![
            "last".to_owned(),
            day_of(SOURCED_LAST_DAY)?.to_string(),
            "-".to_owned(),
            "the calendar of record ends".to_owned(),
            "pull::calendar::SOURCED_LAST_DAY, the last day NSE/CMTR/71775 covers".to_owned(),
        ],
    ];
    for day in SOURCED_FIRST_DAY..=SOURCED_LAST_DAY {
        let kind = exchange_kind_of(day);
        if kind == weekday_rule(day) {
            continue;
        }
        let note = holiday_note(day);
        let special = special_session(day);
        let (kind, windows, name, source) = match kind {
            DayKind::Closed => (
                "holiday",
                "-".to_owned(),
                note.map_or("closed", |n| n.name).to_owned(),
                note.map_or(
                    "measured: pull::calendar's traded-day table (the store's daily rungs)",
                    |n| n.circular,
                )
                .to_owned(),
            ),
            DayKind::Open(session) => (
                "session",
                render_windows(session),
                special
                    .map_or("irregular session", |s| s.kind.label())
                    .to_owned(),
                special
                    .map_or("measured: pull::calendar's irregular-session table", |s| {
                        s.evidence
                    })
                    .to_owned(),
            ),
            DayKind::OpenLengthUnmeasured => (
                "session",
                "unmeasured".to_owned(),
                special
                    .map(|s| s.kind.label())
                    .or_else(|| special_note(day).map(|n| n.name))
                    .unwrap_or("session of unrecorded length")
                    .to_owned(),
                note.or_else(|| special_note(day))
                    .map_or(
                        "pull::calendar: open, length not recorded by any circular read",
                        |n| n.circular,
                    )
                    .to_owned(),
            ),
            DayKind::Unmeasured => {
                return Err(format!("day {day} inside the record is unmeasured"));
            }
        };
        rows.push(vec![
            kind.to_owned(),
            day_of(day)?.to_string(),
            windows,
            field(&name),
            field(&source),
        ]);
    }
    Ok(Draft {
        table: Table::Calendar,
        source: "pull::calendar: NSE/CMTR/36475, NSE/CMTR/39612, NSE/CMTR/40536, NSE/MSD/42319, \
                 NSE/CMTR/71775 and the measured window (D-2805, D-3319, D-3320, D-3346)"
            .to_owned(),
        source_sha256: None,
        fetched: ymd_day((2026, 10, 10))?,
        standing: Standing::Measured,
        decision: "D-2805".to_owned(),
        rows,
    })
}

#[cfg(test)]
#[path = "reference_data_tests.rs"]
mod tests;
