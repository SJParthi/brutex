//! `pull::gdfl_run`: the one GDFL run authority (IA-1; the ingest design's
//! D-4800, D-4801, D-4839, D-4902, D-4903 and D-4904, landed as
//! `docs/05-decisions.md` D-4800, D-4801, D-4839, D-4894, D-4900 to D-4904).
//!
//! # What lives here, and why one place
//!
//! Until IA-1 the setup of a GDFL import was `cli gdfl-import`'s own: which
//! variable names a source, which reader opens it, what the run files
//! (D-3333's stored scope), the `--stored`-with-options refusal and the
//! measure of the archive. The api's import (IA-10a) would have been a
//! second copy of all of it, and a refusal the cli says by one code the api
//! would have said by another. Now both edges build a [`Request`], and
//! [`plan`] and [`execute`] are the one path: the cli keeps only its
//! argument and environment parsing.
//!
//! - [`Request`]: what a run is asked for, with [`Request::to_line`] and
//!   [`Request::from_line`], the one codec of the request line an api child
//!   reads on stdin (D-4902, the design's D-4902: ≤ 8 KiB, the whole filter
//!   and the snapshot name).
//! - [`plan`]: every refusal before any lock, in one order: the request's
//!   misuse ([`RunRefusal::misuse`]), a run whose event sink was refused
//!   (`log_unavailable`, D-4903), a build that may not write
//!   (`unstamped_write`, the answer `cli::store_write` gives, D-4894), a
//!   source root no variable names (`source_unset`), and the scope.
//! - [`execute`]: this process's kinds, in order, on the calling thread: no
//!   thread per kind, ever (round 4, BL-A).
//! - [`ProgressFile`]: the progress record (D-4904), written by
//!   the [`Observer`] a run is handed, at most once a second and at every
//!   day that closed; byte layout in `docs/02-store-format.md` §37.
//! - [`JournalReader`]: the journal read once and refreshed with only its
//!   new bytes (`Journal::refresh`, `docs/06-limits.md` L-IA-2).
//! - [`run_dir`]: the log directory a run of a kind writes to (D-4862,
//!   built by IA-11r as `telemetry::run_dirs`), chosen from the store's
//!   census layout.
//!
//! # What it does not do
//!
//! No lock, claim, runs ledger, stop file or resume point: those are IA-2's
//! (round 4, SF-11), and the lock a cli run takes is still AF-01's door
//! (`cli/src/gdfl_import_guard.rs`). No source but the two the cli has
//! (`zip`, `tickstore`): `dayzips` is IA-3's. No verification mode: only
//! [`Mode::Import`] is built, and a request line naming another is refused.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use brutex_core::vendor::Vendor;

use crate::gdfl_archive::Archive;
use crate::gdfl_cm::CmScope;
use crate::gdfl_import::{
    Durability, ImportKind, ImportRefusal, Journal, Observer, Origin, Refreshed, Report, Run,
    RunOptions, SourceKind, Workers, journal_path, run_cm_from, run_nfo_from,
};
use crate::gdfl_nfo::{NfoTickStore, NfoZips};
use crate::gdfl_tickstore::TickStore;
use crate::session::Day;

/// What a GDFL run files (D-3333), moved here from `cli` (IA-1).
#[path = "gdfl_run_scope.rs"]
pub mod scope;

/// The telemetry target of this module's events.
pub const TARGET: &str = "gdfl.run";

/// The tick store's root.
pub const TICKSTORE_VAR: &str = "BRUTEX_GDFL_TICKSTORE";
/// The vendor's capital-market outer zip.
pub const CM_ARCHIVE_VAR: &str = "BRUTEX_GDFL_CM_ARCHIVE";
/// The directory of the vendor's yearly options zips.
pub const NFO_ARCHIVES_VAR: &str = "BRUTEX_GDFL_NFO_ARCHIVES";

/// The longest request line [`Request::from_line`] reads, and
/// [`Request::to_line`] writes: the form's bound (D-4902).
pub const REQUEST_LINE_MAX: usize = 8 * 1024;

/// The most names a filter may hold (D-4801, R13).
pub const ONLY_MAX: usize = 256;

/// The longest snapshot name, in bytes (D-4902).
pub const SNAPSHOT_MAX: usize = 64;

/// The request line's first word and version.
const REQUEST_HEAD: &str = "gdfl-request v1";

// ═══ The source ═════════════════════════════════════════════════════════════

/// The kind-agnostic word a request names its source by (D-4801), as the
/// cli's `SOURCE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceWord {
    /// The vendor's zips: the capital-market outer zip, or the yearly
    /// options zips.
    Zip,
    /// The verified tick store.
    TickStore,
}

impl SourceWord {
    /// Every word, in the order the cli lists them.
    pub const ALL: [Self; 2] = [Self::TickStore, Self::Zip];

    /// The word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::TickStore => "tickstore",
        }
    }

    /// The word as a source, or `None`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.as_str() == word)
    }
}

/// A source resolved for one kind (D-4801): what claims, plans and progress
/// show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// The capital-market outer zip (indices, stocks).
    CmZip,
    /// The directory of the yearly options zips.
    NfoZips,
    /// The verified tick store (every kind).
    TickStore,
}

impl SourceId {
    /// The id's word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CmZip => "cm_zip",
            Self::NfoZips => "nfo_zips",
            Self::TickStore => "tickstore",
        }
    }

    /// The variable that names its root: no default, no fallback
    /// (`CLAUDE.md` §8's rule for configuration).
    #[must_use]
    pub const fn var(self) -> &'static str {
        match self {
            Self::CmZip => CM_ARCHIVE_VAR,
            Self::NfoZips => NFO_ARCHIVES_VAR,
            Self::TickStore => TICKSTORE_VAR,
        }
    }

    /// How its origin is measured.
    #[must_use]
    pub const fn origin(self) -> SourceKind {
        match self {
            Self::CmZip | Self::NfoZips => SourceKind::Zip,
            Self::TickStore => SourceKind::TickStore,
        }
    }

    /// Its byte in the progress record.
    const fn code(self) -> u8 {
        match self {
            Self::CmZip => 1,
            Self::NfoZips => 2,
            Self::TickStore => 3,
        }
    }
}

/// The source `word` resolves to for `kind`: `zip` is the capital-market
/// outer zip for indices and stocks and the yearly zips for options.
#[must_use]
pub const fn resolve_source(word: SourceWord, kind: ImportKind) -> SourceId {
    match (word, kind) {
        (SourceWord::TickStore, _) => SourceId::TickStore,
        (SourceWord::Zip, ImportKind::Options) => SourceId::NfoZips,
        (SourceWord::Zip, ImportKind::Indices | ImportKind::Stocks) => SourceId::CmZip,
    }
}

/// The root `var` names, from `env`.
///
/// # Errors
///
/// [`RunRefusal::SourceUnset`] when it is unset or empty: named, never
/// defaulted.
pub fn source_root(
    var: &'static str,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, RunRefusal> {
    match env(var) {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(RunRefusal::SourceUnset { var }),
    }
}

// ═══ The request ════════════════════════════════════════════════════════════

/// A run's mode. Only the import is built (IA-1); the design's verify,
/// deferred and completeness modes are IA-6's and IA-7's, and a request line
/// naming one is refused, never read as an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Read the source and file its days.
    Import,
}

impl Mode {
    /// The mode's word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Import => 1,
        }
    }
}

/// Who started a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum By {
    /// A person at a terminal (`cli gdfl-import`).
    Cli,
    /// The app (a press of Ingest, IA-10a).
    Api,
}

impl By {
    /// The word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Api => "api",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Cli => 1,
            Self::Api => 2,
        }
    }
}

/// The kinds a request names: at least one, each once, run in
/// `ImportKind::ALL`'s order (indices, stocks, options) whatever order they
/// were named in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kinds(u8);

impl Kinds {
    /// One kind.
    #[must_use]
    pub const fn one(kind: ImportKind) -> Self {
        Self(bit(kind))
    }

    /// The kinds of `list`.
    ///
    /// # Errors
    ///
    /// [`RunRefusal::RequestLineInvalid`] for an empty list or a kind named
    /// twice.
    pub fn of(list: &[ImportKind]) -> Result<Self, RunRefusal> {
        let mut bits = 0_u8;
        for kind in list {
            if bits & bit(*kind) != 0 {
                return Err(RunRefusal::RequestLineInvalid {
                    why: format!("the kind {} is named twice", kind.as_str()),
                });
            }
            bits |= bit(*kind);
        }
        if bits == 0 {
            return Err(RunRefusal::RequestLineInvalid {
                why: "no kind is named".to_owned(),
            });
        }
        Ok(Self(bits))
    }

    /// The kinds, in run order.
    pub fn iter(self) -> impl Iterator<Item = ImportKind> {
        ImportKind::ALL
            .into_iter()
            .filter(move |kind| self.0 & bit(*kind) != 0)
    }

    /// Whether `kind` is among them.
    #[must_use]
    pub const fn contains(self, kind: ImportKind) -> bool {
        self.0 & bit(kind) != 0
    }

    /// How many.
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Never: a [`Kinds`] holds at least one kind.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// A kind's bit in [`Kinds`].
const fn bit(kind: ImportKind) -> u8 {
    match kind {
        ImportKind::Indices => 1,
        ImportKind::Stocks => 2,
        ImportKind::Options => 4,
    }
}

/// A kind's byte in the progress record.
const fn kind_code(kind: ImportKind) -> u8 {
    match kind {
        ImportKind::Indices => 1,
        ImportKind::Stocks => 2,
        ImportKind::Options => 3,
    }
}

/// What a run is asked to do: everything a run parameter is, and nothing a
/// process knows about itself (its store root, its build, its log; those are
/// [`Process`]). **A run process reads no run parameter from its
/// environment** (D-4901): the cli maps `BRUTEX_GDFL_WORKERS` and
/// `BRUTEX_GDFL_DURABILITY` into this, and an api child takes them from its
/// request line only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The mode.
    pub mode: Mode,
    /// The kinds, run in order.
    pub kinds: Kinds,
    /// The source word, resolved per kind by [`resolve_source`].
    pub source: SourceWord,
    /// The first day, inclusive.
    pub from: Day,
    /// The last day, inclusive.
    pub to: Day,
    /// Only these symbols (spot) or underlyings (options); empty is every
    /// one. At most [`ONLY_MAX`].
    pub only: Vec<String>,
    /// `--stored` (D-3333): narrows a capital-market run to what the stored
    /// scope adds. Refused with options ([`RunRefusal::StoredWithOptions`]).
    pub stored: bool,
    /// Read again the days done with refusals (D-3317 item 2).
    pub retry_refused: bool,
    /// The run's workers (WP-16).
    pub workers: Workers,
    /// When a day is made durable (WP-16, D-3353; D-4901).
    pub durability: Durability,
    /// Who started it.
    pub by: By,
    /// The operator's snapshot name for the run, if any (D-4902): at most
    /// [`SNAPSHOT_MAX`] bytes of letters, digits, `.`, `-` and `_`, beginning
    /// with a letter or a digit.
    pub snapshot: Option<String>,
}

impl Request {
    /// A request of `kinds` over `from..=to` with every other field its
    /// default: the source tick store, no filter, no flag, the host's
    /// workers, the default durability, started by the cli.
    #[must_use]
    pub fn new(kinds: Kinds, source: SourceWord, (from, to): (Day, Day)) -> Self {
        Self {
            mode: Mode::Import,
            kinds,
            source,
            from,
            to,
            only: Vec::new(),
            stored: false,
            retry_refused: false,
            workers: Workers::for_host(),
            durability: Durability::DEFAULT,
            by: By::Cli,
            snapshot: None,
        }
    }

    /// What the request asks that no run may do, before anything is read:
    /// `--stored` with options, a range that runs backwards, an empty name,
    /// a filter of more than [`ONLY_MAX`] names, a name that is not a symbol
    /// as the store files it, a snapshot name outside its alphabet or bound. `None` for a request [`plan`] may go on with.
    #[must_use]
    pub fn misuse(&self) -> Option<RunRefusal> {
        if self.stored && self.kinds.contains(ImportKind::Options) {
            return Some(RunRefusal::StoredWithOptions);
        }
        if self.from > self.to {
            return Some(RunRefusal::RangeBackwards {
                from: self.from,
                to: self.to,
            });
        }
        if self.only.iter().any(|name| name.trim().is_empty()) {
            return Some(RunRefusal::EmptySymbol);
        }
        if self.only.len() > ONLY_MAX {
            return Some(RunRefusal::FilterTooLong {
                names: self.only.len(),
            });
        }
        if let Some(name) = self.only.iter().find(|name| !symbol_ok(name)) {
            return Some(RunRefusal::SymbolInvalid {
                name: shown(name, 32),
            });
        }
        if let Some(name) = &self.snapshot
            && !snapshot_ok(name)
        {
            return Some(RunRefusal::SnapshotInvalid {
                name: shown(name, SNAPSHOT_MAX),
            });
        }
        None
    }

    /// The request as ONE line (D-4902): the fields in a fixed
    /// order, each `name=value`, every value percent-encoded outside letters,
    /// digits, `.`, `-` and `_`, the lists comma-joined. No newline.
    ///
    /// # Errors
    ///
    /// What [`Request::misuse`] refuses, first: a request no run may make is
    /// never written (an empty name would read back as no filter at all).
    /// [`RunRefusal::RequestLineInvalid`] when the line would be longer than
    /// [`REQUEST_LINE_MAX`]: never cut, since a cut filter is another run.
    pub fn to_line(&self) -> Result<String, RunRefusal> {
        if let Some(misuse) = self.misuse() {
            return Err(misuse);
        }
        let kinds: Vec<&str> = self.kinds.iter().map(ImportKind::as_str).collect();
        let only: Vec<String> = self.only.iter().map(|name| encode(name)).collect();
        let line = format!(
            "{REQUEST_HEAD} mode={} kinds={} source={} from={} to={} only={} stored={} \
             retry_refused={} workers={} durability={} by={} snapshot={}",
            self.mode.as_str(),
            kinds.join(","),
            self.source.as_str(),
            self.from,
            self.to,
            only.join(","),
            u8::from(self.stored),
            u8::from(self.retry_refused),
            self.workers.get(),
            self.durability.as_str(),
            self.by.as_str(),
            self.snapshot.as_deref().map_or_else(String::new, encode),
        );
        if line.len() > REQUEST_LINE_MAX {
            return Err(RunRefusal::RequestLineInvalid {
                why: format!(
                    "the request is {} bytes as a line; a line holds at most {REQUEST_LINE_MAX}",
                    line.len()
                ),
            });
        }
        Ok(line)
    }

    /// The request a line [`Request::to_line`] wrote names, byte for byte.
    ///
    /// # Errors
    ///
    /// [`RunRefusal::RequestLineInvalid`] for a line over
    /// [`REQUEST_LINE_MAX`], another head or version, a field missing, out of
    /// order, repeated or unknown, or a value that is not one; then what
    /// [`Request::misuse`] refuses. Refused before any lock.
    pub fn from_line(line: &str) -> Result<Self, RunRefusal> {
        let invalid = |why: String| RunRefusal::RequestLineInvalid { why };
        if line.len() > REQUEST_LINE_MAX {
            return Err(invalid(format!(
                "the line is {} bytes; it holds at most {REQUEST_LINE_MAX}",
                line.len()
            )));
        }
        let line = line.strip_suffix('\n').unwrap_or(line);
        let Some(rest) = line.strip_prefix(REQUEST_HEAD) else {
            return Err(invalid(format!(
                "the line does not begin {REQUEST_HEAD:?}: {}",
                shown(line, 64)
            )));
        };
        let mut fields = rest.split(' ');
        if fields.next() != Some("") {
            return Err(invalid(format!(
                "the head is not {REQUEST_HEAD:?}: {}",
                shown(line, 64)
            )));
        }
        let mut next = |name: &'static str| field_of(&mut fields, name);
        let mode = match next("mode")? {
            "import" => Mode::Import,
            other => {
                return Err(invalid(format!(
                    "the mode {} is not built (only import is)",
                    shown(other, 32)
                )));
            }
        };
        let kinds = next("kinds")?
            .split(',')
            .map(|word| {
                ImportKind::parse(word)
                    .map_err(|_| invalid(format!("{} is not a kind", shown(word, 32))))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let kinds = Kinds::of(&kinds)?;
        let source_word = next("source")?;
        let source = SourceWord::parse(source_word)
            .ok_or_else(|| invalid(format!("{} is not a source", shown(source_word, 32))))?;
        let from = day_of(next("from")?).map_err(invalid)?;
        let to = day_of(next("to")?).map_err(invalid)?;
        let only = match next("only")? {
            "" => Vec::new(),
            list => list
                .split(',')
                .map(|name| decode(name).map_err(invalid))
                .collect::<Result<Vec<_>, _>>()?,
        };
        let stored = flag(next("stored")?).map_err(invalid)?;
        let retry_refused = flag(next("retry_refused")?).map_err(invalid)?;
        let workers_word = next("workers")?;
        let workers = workers_word
            .parse::<usize>()
            .map_err(|_| format!("{} is not a count of workers", shown(workers_word, 32)))
            .and_then(Workers::new)
            .map_err(invalid)?;
        let durability = Durability::parse(next("durability")?).map_err(invalid)?;
        let by = match next("by")? {
            "cli" => By::Cli,
            "api" => By::Api,
            other => return Err(invalid(format!("{} is not who", shown(other, 32)))),
        };
        let snapshot = match next("snapshot")? {
            "" => None,
            name => Some(decode(name).map_err(invalid)?),
        };
        if let Some(extra) = fields.next() {
            return Err(invalid(format!(
                "a field after snapshot: {}",
                shown(extra, 64)
            )));
        }
        let request = Self {
            mode,
            kinds,
            source,
            from,
            to,
            only,
            stored,
            retry_refused,
            workers,
            durability,
            by,
            snapshot,
        };
        request.misuse().map_or(Ok(request), Err)
    }
}

/// The next field of a request line, which must be `name=`: its value.
fn field_of<'l>(
    fields: &mut std::str::Split<'l, char>,
    name: &'static str,
) -> Result<&'l str, RunRefusal> {
    let invalid = |why: String| RunRefusal::RequestLineInvalid { why };
    let Some(field) = fields.next() else {
        return Err(invalid(format!("the field {name} is missing")));
    };
    field
        .strip_prefix(name)
        .and_then(|after| after.strip_prefix('='))
        .ok_or_else(|| {
            invalid(format!(
                "expected the field {name}, found {}",
                shown(field, 64)
            ))
        })
}

/// Whether `name` is a snapshot name (D-4902): it begins with a letter or a
/// digit (IA-1 repair 1), so a word shaped like a flag (`--stored`) or a
/// path step (`.`, `..`) is never one.
fn snapshot_ok(name: &str) -> bool {
    name.bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric())
        && name.len() <= SNAPSHOT_MAX
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Whether `name` is a symbol exactly as the store files it: the rule the
/// importer's `check_filter` applies after the lock (D-2811), applied here
/// before it.
fn symbol_ok(name: &str) -> bool {
    brutex_core::symbol::Symbol::new(name).is_ok_and(|symbol| symbol.as_str() == name)
}

/// `0` or `1`.
fn flag(word: &str) -> Result<bool, String> {
    match word {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(format!("{} is not 0 or 1", shown(other, 32))),
    }
}

/// `YYYY-MM-DD` as a day.
fn day_of(text: &str) -> Result<Day, String> {
    let bad = || format!("{} is not a YYYY-MM-DD day", shown(text, 32));
    let parts: Vec<&str> = text.split('-').collect();
    let number = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let [y, m, d] = parts.as_slice() else {
        return Err(bad());
    };
    let (Some(y), Some(m), Some(d)) = (number(y, 4), number(m, 2), number(d, 2)) else {
        return Err(bad());
    };
    let (Ok(m), Ok(d)) = (u8::try_from(m), u8::try_from(d)) else {
        return Err(bad());
    };
    Day::new(y, m, d).map_err(|why| format!("{text}: {why}"))
}

/// `text` percent-encoded: every byte outside letters, digits, `.`, `-` and
/// `_` as `%XX` (upper-case hex), so no value holds a space, a comma, an
/// `=`, a newline or a byte that is not ASCII.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
    }
    out
}

/// One hex digit, upper case.
fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        _ => char::from(b'A' + nibble - 10),
    }
}

/// What [`encode`] wrote, decoded; refused when it is not exactly that: a
/// raw byte [`encode`] would have encoded, a `%` not followed by two
/// upper-case hex digits, or bytes that are not UTF-8.
fn decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            let digit = |i: usize| {
                bytes.get(i).and_then(|b| {
                    char::from(*b)
                        .to_digit(16)
                        .filter(|_| !b.is_ascii_lowercase())
                })
            };
            let (Some(hi), Some(lo)) = (digit(at + 1), digit(at + 2)) else {
                return Err(format!("{} holds a bad %-escape", shown(text, 64)));
            };
            out.push(u8::try_from(hi * 16 + lo).unwrap_or(0));
            at += 3;
        } else if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
            out.push(byte);
            at += 1;
        } else {
            return Err(format!(
                "{} holds a byte that must be escaped",
                shown(text, 64)
            ));
        }
    }
    String::from_utf8(out).map_err(|_| format!("{} is not UTF-8 once decoded", shown(text, 64)))
}

/// `text` as a refusal shows it: at most `cap` bytes, cut on a whole
/// character, debug-quoted so a control character is visible.
fn shown(text: &str, cap: usize) -> String {
    format!("{:?}", cut(text, cap))
}

/// `text` cut to at most `cap` bytes on a whole character (RA-307's rule).
fn cut(text: &str, cap: usize) -> &str {
    if text.len() <= cap {
        return text;
    }
    let mut end = cap;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or("")
}

// ═══ Refusals ═══════════════════════════════════════════════════════════════

/// Why a run is refused before its first day (codes are never renamed or
/// reused; design §5.8). Every one is refused before any lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRefusal {
    /// The request line could not be read (D-4902).
    RequestLineInvalid {
        /// Why.
        why: String,
    },
    /// `--stored` with options (D-3333; one code from every edge, IA-GR-13).
    StoredWithOptions,
    /// The range runs backwards.
    RangeBackwards {
        /// The first day.
        from: Day,
        /// The last day.
        to: Day,
    },
    /// A filter name that is empty.
    EmptySymbol,
    /// A filter of more than [`ONLY_MAX`] names.
    FilterTooLong {
        /// How many.
        names: usize,
    },
    /// A snapshot name outside its alphabet or bound.
    SnapshotInvalid {
        /// The name, as shown.
        name: String,
    },
    /// The run's event sink was refused (D-4903): no run that
    /// writes the store works unlogged.
    LogUnavailable {
        /// Why, in the sink's own words.
        why: String,
    },
    /// This build may not write the store (DB-B71; `cli::store_write`'s
    /// answer, D-4894).
    Unstamped {
        /// Why, in `cli::store_write`'s words.
        why: String,
    },
    /// No variable names the source's root (`CLAUDE.md` §8).
    SourceUnset {
        /// The variable.
        var: &'static str,
    },
    /// The scope could not be built (D-3333).
    Scope {
        /// Why.
        why: String,
    },
    /// The source could not be opened.
    SourceUnopened {
        /// The root.
        path: PathBuf,
        /// Why.
        why: String,
    },
    /// A filter name that is not a symbol as the store files it (IA-1 repair
    /// 1): refused here, before any lock, by the rule the importer's
    /// `filter_name` applies after it (`brutex_core::symbol::Symbol`, exact
    /// bytes).
    SymbolInvalid {
        /// The name, as shown.
        name: String,
    },
}

impl RunRefusal {
    /// The refusal's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestLineInvalid { .. } => "request_line_invalid",
            Self::StoredWithOptions => "stored_with_options",
            Self::RangeBackwards { .. } => "range_backwards",
            Self::EmptySymbol => "symbol_empty",
            Self::FilterTooLong { .. } => "filter_too_long",
            Self::SnapshotInvalid { .. } => "snapshot_invalid",
            Self::LogUnavailable { .. } => "log_unavailable",
            Self::Unstamped { .. } => "unstamped_write",
            Self::SourceUnset { .. } => "source_unset",
            Self::Scope { .. } => "scope_unreadable",
            Self::SourceUnopened { .. } => "source_unopened",
            Self::SymbolInvalid { .. } => "symbol_invalid",
        }
    }

    /// Whether the request itself is wrong (the cli exits `MISUSED`, the api
    /// answers 400), rather than the process or its data (`FAILED`, 503).
    #[must_use]
    pub const fn misuse(&self) -> bool {
        matches!(
            self,
            Self::RequestLineInvalid { .. }
                | Self::StoredWithOptions
                | Self::RangeBackwards { .. }
                | Self::EmptySymbol
                | Self::FilterTooLong { .. }
                | Self::SnapshotInvalid { .. }
                | Self::SymbolInvalid { .. }
        )
    }

    /// Every code, in the order of the enum: the table a reader names a code
    /// by, and the test that no two variants share one.
    pub const CODES: [&'static str; 12] = [
        "request_line_invalid",
        "stored_with_options",
        "range_backwards",
        "symbol_empty",
        "filter_too_long",
        "snapshot_invalid",
        "log_unavailable",
        "unstamped_write",
        "source_unset",
        "scope_unreadable",
        "source_unopened",
        "symbol_invalid",
    ];
}

impl core::fmt::Display for RunRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Self::RequestLineInvalid { why } => write!(f, "the request line is not one: {why}"),
            Self::StoredWithOptions => f.write_str(
                "--stored narrows a capital-market run; an options run files no stored series",
            ),
            Self::RangeBackwards { from, to } => {
                write!(f, "the range runs backwards: {from} is after {to}")
            }
            Self::EmptySymbol => {
                f.write_str("an empty SYMBOL names no instrument; give names or none")
            }
            Self::FilterTooLong { names } => write!(
                f,
                "the filter names {names} instruments; a run names at most {ONLY_MAX}"
            ),
            Self::SnapshotInvalid { name } => write!(
                f,
                "the snapshot name {name} is not 1 to {SNAPSHOT_MAX} letters, digits, '.', '-' \
                 or '_' beginning with a letter or a digit (a flag or a path step is no name)"
            ),
            Self::LogUnavailable { why } => write!(
                f,
                "this run's events could not be recorded ({why}), and a run that writes the \
                 store does not run unlogged (D-4903): nothing was read or written"
            ),
            Self::Unstamped { why } | Self::Scope { why } => f.write_str(why),
            Self::SourceUnset { var } => write!(
                f,
                "{var} is not set; it names where the GDFL data is read from and has no default"
            ),
            Self::SourceUnopened { path, why } => write!(f, "{}: {why}", path.display()),
            Self::SymbolInvalid { name } => write!(
                f,
                "{name} is not a symbol as the store files it (A-Z, 0-9, '-', '_' or '&', at \
                 most {cap} bytes): refused before any lock, as the importer would refuse \
                 it after one",
                cap = brutex_core::symbol::SYMBOL_CAPACITY
            ),
        }
    }
}

// ═══ The process ════════════════════════════════════════════════════════════

/// Whether this process's event sink is installed (D-4903).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogState {
    /// Installed, writing to this directory.
    Installed(PathBuf),
    /// Refused, in the sink's own words.
    Refused(String),
}

impl LogState {
    /// The process-wide sink's state: installed in its directory, or not
    /// installed, with `why` as the reason the binary gave.
    #[must_use]
    pub fn of_process(why: &str) -> Self {
        telemetry::global().map_or_else(
            || Self::Refused(why.to_owned()),
            |sink| Self::Installed(sink.dir().to_path_buf()),
        )
    }
}

/// What the process running a request knows about itself: none of it is a
/// run parameter, so none of it is in the request line.
#[derive(Debug, Clone, Copy)]
pub struct Process<'a> {
    /// The store root it writes.
    pub store_root: &'a Path,
    /// Its build's commit stamp, or `None` for an unstamped build.
    pub commit: Option<&'a str>,
    /// Its event sink.
    pub log: &'a LogState,
    /// `Some(why)` when this build may not write the store
    /// (`cli::store_write::unstamped_write_refusal`'s answer, D-4894).
    pub unstamped: Option<&'a str>,
}

// ═══ The plan ═══════════════════════════════════════════════════════════════

/// What one kind of a planned run reads, files and from where.
#[derive(Debug, Clone)]
pub struct KindPlan {
    /// The kind.
    pub kind: ImportKind,
    /// Its resolved source.
    pub source: SourceId,
    /// The source's root.
    pub root: PathBuf,
    /// What it files (D-3333).
    pub scope: Arc<CmScope>,
    /// The receipt's sentence for a stored scope; `None` for options.
    pub scope_words: Option<String>,
    /// Its filter: the request's, and for `--stored` what the scope adds.
    pub only: Vec<String>,
    /// The source as measured, with this build's commit (D-3317 item 4).
    pub origin: Origin,
}

/// A request every pre-lock check has passed, with each kind resolved.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The request.
    pub request: Request,
    /// The store root.
    pub store_root: PathBuf,
    /// Each kind, in run order.
    pub kinds: Vec<KindPlan>,
}

/// Plans `request` in `process` (IA-1): every refusal before any lock, in
/// one order, then each kind's source, scope and origin.
///
/// # Errors
///
/// In order: [`Request::misuse`]'s refusal, [`RunRefusal::LogUnavailable`],
/// [`RunRefusal::Unstamped`], then per kind [`RunRefusal::SourceUnset`] and
/// [`RunRefusal::Scope`]. Nothing is written and no lock is taken on any of
/// them.
pub fn plan(
    request: Request,
    process: &Process<'_>,
    env: &dyn Fn(&str) -> Option<OsString>,
    cache: &mut SourceCache,
) -> Result<Plan, RunRefusal> {
    if let Some(misuse) = request.misuse() {
        return Err(misuse);
    }
    if let LogState::Refused(why) = process.log {
        return Err(RunRefusal::LogUnavailable { why: why.clone() });
    }
    if let Some(why) = process.unstamped {
        return Err(RunRefusal::Unstamped {
            why: why.to_owned(),
        });
    }
    let mut kinds = Vec::with_capacity(request.kinds.len());
    for kind in request.kinds.iter() {
        let source = resolve_source(request.source, kind);
        let root = source_root(source.var(), env)?;
        let mut only = request.only.clone();
        let (scope, scope_words) =
            scope::for_request(kind, process.store_root, request.stored, &mut only)
                .map_err(|why| RunRefusal::Scope { why })?;
        let origin = cache.origin(source, &root, process.commit);
        kinds.push(KindPlan {
            kind,
            source,
            root,
            scope,
            scope_words,
            only,
            origin,
        });
    }
    Ok(Plan {
        request,
        store_root: process.store_root.to_path_buf(),
        kinds,
    })
}

/// A source root's identity, as the cache keys its measure by: the root's
/// device, inode, modification time and length (design §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RootIdentity {
    dev: u64,
    ino: u64,
    mtime: i64,
    mtime_ns: i64,
    len: u64,
}

impl RootIdentity {
    fn of(root: &Path) -> Option<Self> {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::metadata(root).ok()?;
        Some(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            mtime: meta.mtime(),
            mtime_ns: meta.mtime_nsec(),
            len: meta.len(),
        })
    }
}

/// One measured source.
#[derive(Debug, Clone)]
struct Measured {
    source: SourceId,
    root: PathBuf,
    identity: RootIdentity,
    commit: Option<String>,
    origin: Origin,
}

/// Each source's measured origin, kept while its root is unchanged (design
/// §5.1): a run process measures once per run; the api (IA-10a) again only
/// when a root's (device, inode, mtime, length) changes. At most one entry a
/// source id, so a lookup is a walk of at most three.
#[derive(Debug, Default)]
pub struct SourceCache {
    held: Vec<Measured>,
    measures: u64,
}

impl SourceCache {
    /// The origin of `source` at `root` for a build stamped `commit`,
    /// measured when this cache holds none for that root, identity and
    /// commit, and its `started` this moment either way (a run's start is
    /// its own, never a cached one). A root whose identity cannot be read is
    /// measured every time, and the measure names why.
    pub fn origin(&mut self, source: SourceId, root: &Path, commit: Option<&str>) -> Origin {
        let identity = RootIdentity::of(root);
        if let Some(identity) = identity
            && let Some(held) = self.held.iter().find(|held| {
                held.source == source
                    && held.root == root
                    && held.identity == identity
                    && held.commit.as_deref() == commit
            })
        {
            let mut origin = held.origin.clone();
            origin.started = now_unix_s();
            return origin;
        }
        let origin = Origin::measure(source.origin(), root, commit);
        self.measures = self.measures.saturating_add(1);
        self.held.retain(|held| held.source != source);
        if let Some(identity) = identity {
            self.held.push(Measured {
                source,
                root: root.to_path_buf(),
                identity,
                commit: commit.map(str::to_owned),
                origin: origin.clone(),
            });
        }
        origin
    }

    /// How many times this cache measured a source.
    #[must_use]
    pub const fn measures(&self) -> u64 {
        self.measures
    }
}

/// The UNIX second now; 0 for a clock before 1970.
fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

// ═══ Execution ══════════════════════════════════════════════════════════════

/// How one kind of a run ended.
#[derive(Debug)]
pub enum KindEnd {
    /// It ran: its report (failures named in it), boxed: a report is
    /// hundreds of bytes beside a refusal's few.
    Report(Box<Report>),
    /// The importer refused it (a census refusal names what it did first).
    Refused(ImportRefusal),
    /// Its source could not be opened.
    Unopened(RunRefusal),
}

/// Runs this process's kinds of `plan` in order, on the calling thread (no
/// thread per kind, ever: round 4, BL-A), each told to `observer`. A kind
/// that does not end in a report stops the run: the kinds after it are not
/// begun and are not in the answer.
#[must_use]
pub fn execute(plan: &Plan, observer: Option<&dyn Observer>) -> Vec<(ImportKind, KindEnd)> {
    let mut ends = Vec::with_capacity(plan.kinds.len());
    for kind in &plan.kinds {
        let end = execute_kind(plan, kind, observer);
        let stop = !matches!(end, KindEnd::Report(_));
        ends.push((kind.kind, end));
        if stop {
            break;
        }
    }
    ends
}

/// One kind of `plan`, as [`execute`] runs it.
#[must_use]
pub fn execute_kind(plan: &Plan, kind: &KindPlan, observer: Option<&dyn Observer>) -> KindEnd {
    let run = Run {
        kind: kind.kind,
        from: plan.request.from,
        to: plan.request.to,
        only: &kind.only,
        store_root: &plan.store_root,
    };
    let options = RunOptions {
        origin: Some(&kind.origin),
        retry_refused: plan.request.retry_refused,
        workers: plan.request.workers,
        durability: plan.request.durability,
        observer,
    };
    let scope = || Arc::clone(&kind.scope);
    let ended = match (kind.source, kind.kind) {
        (SourceId::TickStore, ImportKind::Options) => {
            run_nfo_from(&NfoTickStore::new(&kind.root), &run, options)
        }
        (SourceId::TickStore, _) => run_cm_from(
            &TickStore::new(&kind.root).with_scope(scope()),
            &run,
            options,
        ),
        (SourceId::NfoZips, _) => run_nfo_from(&NfoZips::new(&kind.root), &run, options),
        (SourceId::CmZip, _) => match Archive::open(&kind.root) {
            Ok(archive) => run_cm_from(&archive.with_scope(scope()), &run, options),
            Err(why) => {
                return KindEnd::Unopened(RunRefusal::SourceUnopened {
                    path: kind.root.clone(),
                    why: why.to_string(),
                });
            }
        },
    };
    match ended {
        Ok(report) => KindEnd::Report(Box::new(report)),
        Err(refusal) => KindEnd::Refused(refusal),
    }
}

// ═══ Where a run logs ═══════════════════════════════════════════════════════

/// The census layout a GDFL run of `store` finds, as far as it decides the
/// run's lock and its log directory.
///
/// # Errors
///
/// The census's own words when it cannot be located: the caller logs to
/// `logs/cli` and lets the run's door refuse, never guessing a layout
/// (critic round 7, IA-1).
pub fn layout_of(store: &Path) -> Result<telemetry::run_dirs::GdflLayout, String> {
    use crate::ingest::census_store::{Location, locate};
    match locate(store, Vendor::Gdfl)? {
        Location::Single(_) => Ok(telemetry::run_dirs::GdflLayout::V3),
        Location::Partitioned(_) => Ok(telemetry::run_dirs::GdflLayout::V4),
    }
}

/// The directory a GDFL run of `kind` on `store` logs to (D-4862): its
/// kind's own on a v4 store, the store lock's on v3.
///
/// # Errors
///
/// What [`layout_of`] refuses.
pub fn run_dir(store: &Path, kind: ImportKind) -> Result<telemetry::run_dirs::RunDir, String> {
    let layout = layout_of(store)?;
    telemetry::run_dirs::for_gdfl(kind.as_str(), layout).ok_or_else(|| {
        format!(
            "{} has no run directory in telemetry::run_dirs (D-4897)",
            kind.as_str()
        )
    })
}

// ═══ The journal, read and refreshed ════════════════════════════════════════

/// The import journal as a reader holds it (IA-1): read once, then only the
/// bytes appended since (`docs/06-limits.md` L-IA-2). Writes nothing, so a
/// reader beside a run never closes the run's torn tail.
#[derive(Debug)]
pub struct JournalReader {
    journal: Journal,
}

impl JournalReader {
    /// Reads the journal of `store_root` whole.
    ///
    /// # Errors
    ///
    /// The journal's refusal (a file that cannot be read, or a line it does
    /// not write).
    pub fn open(store_root: &Path) -> Result<Self, ImportRefusal> {
        Ok(Self {
            journal: Journal::read_only(journal_path(store_root))?,
        })
    }

    /// Reads what was appended since (`Journal::refresh`).
    ///
    /// # Errors
    ///
    /// As [`JournalReader::open`]; after one the next refresh reads the
    /// whole file again.
    pub fn refresh(&mut self) -> Result<Refreshed, ImportRefusal> {
        self.journal.refresh()
    }

    /// Whether `kind`'s `day` is closed `done` under `filter` (`*` for the
    /// whole tree) by this build's bar definition: one hash probe.
    #[must_use]
    pub fn is_done(&self, kind: ImportKind, day: Day, filter: &str) -> bool {
        self.journal
            .is_done(&format!("{} {day} {filter}", kind.as_str()))
    }

    /// How many runs the journal names, and how many keys are begun and not
    /// closed.
    #[must_use]
    pub fn runs_and_open(&self) -> (usize, usize) {
        self.journal.runs_and_open()
    }

    /// The bytes of whole lines read.
    #[must_use]
    pub const fn loaded(&self) -> u64 {
        self.journal.loaded()
    }
}

// ═══ The progress record (D-4904; docs/02-store-format.md §37) ═════

/// The progress record's length: one 4 KiB positional write.
pub const PROGRESS_BYTES: usize = 4_096;
/// Its magic.
pub const PROGRESS_MAGIC: [u8; 8] = *b"BRUTEXPG";
/// The format version this build writes and reads.
pub const PROGRESS_VERSION: u16 = 1;
/// The refusal slots.
pub const PROGRESS_SLOTS: usize = 8;
/// One slot's length.
pub const SLOT_BYTES: usize = 352;
/// Where the slots begin.
pub const SLOTS_AT: usize = 256;
/// Where the CRC-32C over every byte before it sits.
pub const CRC_AT: usize = PROGRESS_BYTES - 4;
/// A slot's field bounds, in bytes: stage, code, about, why.
pub const SLOT_STAGE: usize = 8;
/// See [`SLOT_STAGE`].
pub const SLOT_CODE: usize = 32;
/// See [`SLOT_STAGE`]. The design's 96 is cut to fit the critic's 352-byte
/// slot beside a 240-byte why.
pub const SLOT_ABOUT: usize = 64;
/// See [`SLOT_STAGE`].
pub const SLOT_WHY: usize = 240;

/// What a run is doing, as its progress record says (design §5.7). The
/// numbers are the record's and are never reused; IA-1 writes `Running`,
/// `Finished`, `Refused`, `Census` and `Durability`; the others are the stop
/// and loss states IA-2 and IA-16 write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunState {
    /// Reading days.
    Running,
    /// A stop was asked (IA-2).
    Stopping,
    /// Every day of the range passed.
    Finished,
    /// Stopped at a file boundary (IA-2).
    Stopped,
    /// Stopped by the server's shutdown (IA-2).
    Shutdown,
    /// The source went away (IA-16).
    SourceLost,
    /// The disk filled (IA-16).
    DiskFull,
    /// Refused by the importer (the journal, the filter, the lineage).
    Refused,
    /// Stopped by a census refusal (D-3317 item 7).
    Census,
    /// Stopped with a day that could not be made durable (D-4901).
    Durability,
}

impl RunState {
    /// Every state, by its byte.
    pub const ALL: [Self; 10] = [
        Self::Running,
        Self::Stopping,
        Self::Finished,
        Self::Stopped,
        Self::Shutdown,
        Self::SourceLost,
        Self::DiskFull,
        Self::Refused,
        Self::Census,
        Self::Durability,
    ];

    /// The state's word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Finished => "finished",
            Self::Stopped => "stopped",
            Self::Shutdown => "shutdown",
            Self::SourceLost => "source_lost",
            Self::DiskFull => "disk_full",
            Self::Refused => "refused",
            Self::Census => "census",
            Self::Durability => "durability",
        }
    }

    /// The state's byte: its place in [`RunState::ALL`], from 1.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Running => 1,
            Self::Stopping => 2,
            Self::Finished => 3,
            Self::Stopped => 4,
            Self::Shutdown => 5,
            Self::SourceLost => 6,
            Self::DiskFull => 7,
            Self::Refused => 8,
            Self::Census => 9,
            Self::Durability => 10,
        }
    }

    /// The state a byte names, or `None` for one this build does not know.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.code() == code)
    }
}

/// One refusal as the progress record keeps it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgressRefusal {
    /// Its day, `yyyymmdd` (0 when it names none).
    pub day: u32,
    /// Its stage word, at most [`SLOT_STAGE`] bytes.
    pub stage: String,
    /// Its code, at most [`SLOT_CODE`] bytes.
    pub code: String,
    /// What it is about, at most [`SLOT_ABOUT`] bytes.
    pub about: String,
    /// Why, at most [`SLOT_WHY`] bytes.
    pub why: String,
}

/// The progress record, decoded (format v1; the offsets are the definition,
/// `docs/02-store-format.md` §37).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// The state's byte ([`RunState::code`]); 0 never written.
    pub state: u8,
    /// The mode's byte (1 import).
    pub mode: u8,
    /// The kind's byte (1 indices, 2 stocks, 3 options).
    pub kind: u8,
    /// The source's byte (1 `cm_zip`, 2 `nfo_zips`, 3 tickstore).
    pub source: u8,
    /// Who started it (1 cli, 2 api).
    pub by: u8,
    /// The census layout (3 or 4; 0 not known).
    pub layout: u8,
    /// The run's journal id (0 before its first day).
    pub run: u64,
    /// The process.
    pub pid: u32,
    /// Its workers.
    pub workers: u32,
    /// The range, `yyyymmdd`.
    pub from: u32,
    /// See [`Progress::from`].
    pub to: u32,
    /// The day now, `yyyymmdd` (0 before the first).
    pub day_now: u32,
    /// The day now's place in the range, from 0.
    pub day_index: u32,
    /// The calendar days of the range.
    pub day_total: u32,
    /// The report's counters, as the cli's receipt names them.
    pub files: u64,
    /// See [`Progress::files`].
    pub files_skipped: u64,
    /// See [`Progress::files`].
    pub files_refused: u64,
    /// See [`Progress::files`].
    pub rows: u64,
    /// See [`Progress::files`].
    pub seconds_written: u64,
    /// Every failure named (D-4839).
    pub refusals_total: u64,
    /// Days by how they ended, as the receipt names them.
    pub days_imported: u64,
    /// See [`Progress::days_imported`].
    pub days_with_refusals: u64,
    /// See [`Progress::days_imported`].
    pub days_incomplete: u64,
    /// See [`Progress::days_imported`].
    pub days_all_refused: u64,
    /// See [`Progress::days_imported`].
    pub days_skipped: u64,
    /// See [`Progress::days_imported`].
    pub days_closed: u64,
    /// See [`Progress::days_imported`].
    pub days_not_held: u64,
    /// See [`Progress::days_imported`].
    pub days_refused: u64,
    /// Seconds since the run began, from a monotonic clock (IA-GR-28).
    pub elapsed_s: u64,
    /// The UNIX second the record began (0 for a clock before 1970).
    pub started: u64,
    /// The UNIX second of this write.
    pub updated: u64,
    /// Events the sink stamped with a held clock since it opened
    /// (`telemetry::Health::clock_held`, RA-224).
    pub clock_held: u64,
    /// The days passed in the rate's window, and the window in seconds:
    /// the whole run so far (a reader divides in integers).
    pub window_days: u64,
    /// See [`Progress::window_days`].
    pub window_s: u64,
    /// Events the run's sink dropped since it opened (`telemetry::Health::dropped`,
    /// RA-202): a run whose log starts losing lines half-way says so here,
    /// beside its counters, and never finishes looking fully logged.
    pub log_dropped: u64,
    /// The sink's failed rolls since it opened
    /// (`telemetry::Health::rotation_failures`, RA-202).
    pub log_failures: u64,
    /// The newest refusals, oldest first, at most [`PROGRESS_SLOTS`].
    pub refusals: Vec<ProgressRefusal>,
}

/// Why a progress record could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressFault {
    /// Not 4,096 bytes.
    Length(usize),
    /// Not `BRUTEXPG`.
    Magic,
    /// Another format version: `progress: format <n> unknown`, never
    /// guessed at.
    Version(u16),
    /// The CRC does not match: a torn read during a write, or damage.
    Crc,
    /// A slot count or a text that is not one.
    Field(&'static str),
}

impl core::fmt::Display for ProgressFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Length(n) => write!(f, "progress: {n} bytes, not {PROGRESS_BYTES}"),
            Self::Magic => f.write_str("progress: not a progress record"),
            Self::Version(n) => write!(f, "progress: format {n} unknown"),
            Self::Crc => f.write_str("progress: CRC mismatch (torn or damaged)"),
            Self::Field(what) => write!(f, "progress: {what} is not one"),
        }
    }
}

/// Little-endian writes into a record.
struct Put<'b>(&'b mut [u8; PROGRESS_BYTES]);

impl Put<'_> {
    fn bytes(&mut self, at: usize, bytes: &[u8]) {
        if let Some(slot) = self.0.get_mut(at..at + bytes.len()) {
            slot.copy_from_slice(bytes);
        }
    }
    fn u8(&mut self, at: usize, value: u8) {
        self.bytes(at, &[value]);
    }
    fn u16(&mut self, at: usize, value: u16) {
        self.bytes(at, &value.to_le_bytes());
    }
    fn u32(&mut self, at: usize, value: u32) {
        self.bytes(at, &value.to_le_bytes());
    }
    fn u64(&mut self, at: usize, value: u64) {
        self.bytes(at, &value.to_le_bytes());
    }
    fn text(&mut self, (len_at, at, cap): (usize, usize, usize), text: &str) {
        let kept = cut(text, cap);
        self.u8(len_at, u8::try_from(kept.len()).unwrap_or(u8::MAX));
        self.bytes(at, kept.as_bytes());
    }
}

/// Little-endian reads from a record.
struct Get<'b>(&'b [u8]);

impl Get<'_> {
    fn array<const N: usize>(&self, at: usize) -> [u8; N] {
        self.0
            .get(at..at + N)
            .and_then(|slice| <[u8; N]>::try_from(slice).ok())
            .unwrap_or([0; N])
    }
    fn u8(&self, at: usize) -> u8 {
        self.0.get(at).copied().unwrap_or(0)
    }
    fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes(self.array(at))
    }
    fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.array(at))
    }
    fn u64(&self, at: usize) -> u64 {
        u64::from_le_bytes(self.array(at))
    }
    fn text(&self, (len_at, at, cap): (usize, usize, usize)) -> Result<String, ProgressFault> {
        let len = usize::from(self.u8(len_at));
        if len > cap {
            return Err(ProgressFault::Field("a slot's text length"));
        }
        let bytes = self
            .0
            .get(at..at + len)
            .ok_or(ProgressFault::Field("a slot's text"))?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ProgressFault::Field("a slot's text"))
    }
}

/// The offsets of the fixed fields (format v1).
mod at {
    pub(super) const VERSION: usize = 8;
    pub(super) const STATE: usize = 10;
    pub(super) const MODE: usize = 11;
    pub(super) const KIND: usize = 12;
    pub(super) const SOURCE: usize = 13;
    pub(super) const BY: usize = 14;
    pub(super) const LAYOUT: usize = 15;
    pub(super) const RUN: usize = 16;
    pub(super) const PID: usize = 24;
    pub(super) const WORKERS: usize = 28;
    pub(super) const FROM: usize = 32;
    pub(super) const TO: usize = 36;
    pub(super) const DAY_NOW: usize = 40;
    pub(super) const DAY_INDEX: usize = 44;
    pub(super) const DAY_TOTAL: usize = 48;
    pub(super) const SLOTS_USED: usize = 52;
    /// The u64 counters, eight bytes each, from here, in `COUNTERS` order.
    pub(super) const COUNTERS: usize = 56;
    pub(super) const COUNTER_COUNT: usize = 22;
}

/// A slot's offsets, from its start: day, the four lengths, the four texts.
const SLOT_DAY: usize = 0;
const SLOT_LENS: usize = 4;
const SLOT_TEXTS: [(usize, usize); 4] = [
    (8, SLOT_STAGE),
    (8 + SLOT_STAGE, SLOT_CODE),
    (8 + SLOT_STAGE + SLOT_CODE, SLOT_ABOUT),
    (8 + SLOT_STAGE + SLOT_CODE + SLOT_ABOUT, SLOT_WHY),
];

impl Progress {
    /// The 22 counters, in their record order (offset 56, 8 bytes each).
    const fn counters(&self) -> [u64; at::COUNTER_COUNT] {
        [
            self.files,
            self.files_skipped,
            self.files_refused,
            self.rows,
            self.seconds_written,
            self.refusals_total,
            self.days_imported,
            self.days_with_refusals,
            self.days_incomplete,
            self.days_all_refused,
            self.days_skipped,
            self.days_closed,
            self.days_not_held,
            self.days_refused,
            self.elapsed_s,
            self.started,
            self.updated,
            self.clock_held,
            self.window_days,
            self.window_s,
            self.log_dropped,
            self.log_failures,
        ]
    }

    /// The record's bytes (format v1).
    #[must_use]
    pub fn encode(&self) -> [u8; PROGRESS_BYTES] {
        let mut bytes = [0_u8; PROGRESS_BYTES];
        let mut put = Put(&mut bytes);
        put.bytes(0, &PROGRESS_MAGIC);
        put.u16(at::VERSION, PROGRESS_VERSION);
        put.u8(at::STATE, self.state);
        put.u8(at::MODE, self.mode);
        put.u8(at::KIND, self.kind);
        put.u8(at::SOURCE, self.source);
        put.u8(at::BY, self.by);
        put.u8(at::LAYOUT, self.layout);
        put.u64(at::RUN, self.run);
        put.u32(at::PID, self.pid);
        put.u32(at::WORKERS, self.workers);
        put.u32(at::FROM, self.from);
        put.u32(at::TO, self.to);
        put.u32(at::DAY_NOW, self.day_now);
        put.u32(at::DAY_INDEX, self.day_index);
        put.u32(at::DAY_TOTAL, self.day_total);
        let kept = self
            .refusals
            .get(self.refusals.len().saturating_sub(PROGRESS_SLOTS)..)
            .unwrap_or_default();
        put.u8(at::SLOTS_USED, u8::try_from(kept.len()).unwrap_or(0));
        for (i, value) in self.counters().into_iter().enumerate() {
            put.u64(at::COUNTERS + 8 * i, value);
        }
        for (i, refusal) in kept.iter().enumerate() {
            let base = SLOTS_AT + i * SLOT_BYTES;
            put.u32(base + SLOT_DAY, refusal.day);
            let texts = [&refusal.stage, &refusal.code, &refusal.about, &refusal.why];
            for (j, ((offset, cap), text)) in SLOT_TEXTS.into_iter().zip(texts).enumerate() {
                put.text((base + SLOT_LENS + j, base + offset, cap), text);
            }
        }
        let crc = store::crc::crc32c(&bytes[..CRC_AT]);
        Put(&mut bytes).u32(CRC_AT, crc);
        bytes
    }

    /// The record `bytes` hold.
    ///
    /// # Errors
    ///
    /// [`ProgressFault`]: the length, the magic, another version (never
    /// guessed at), the CRC, or a slot that is not one.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProgressFault> {
        if bytes.len() != PROGRESS_BYTES {
            return Err(ProgressFault::Length(bytes.len()));
        }
        let get = Get(bytes);
        if get.array::<8>(0) != PROGRESS_MAGIC {
            return Err(ProgressFault::Magic);
        }
        let version = get.u16(at::VERSION);
        if version != PROGRESS_VERSION {
            return Err(ProgressFault::Version(version));
        }
        if store::crc::crc32c(bytes.get(..CRC_AT).unwrap_or_default()) != get.u32(CRC_AT) {
            return Err(ProgressFault::Crc);
        }
        let used = usize::from(get.u8(at::SLOTS_USED));
        if used > PROGRESS_SLOTS {
            return Err(ProgressFault::Field("the slot count"));
        }
        let counter = |i: usize| get.u64(at::COUNTERS + 8 * i);
        let mut refusals = Vec::with_capacity(used);
        for i in 0..used {
            let base = SLOTS_AT + i * SLOT_BYTES;
            let mut texts = Vec::with_capacity(4);
            for (j, (offset, cap)) in SLOT_TEXTS.into_iter().enumerate() {
                texts.push(get.text((base + SLOT_LENS + j, base + offset, cap))?);
            }
            let [stage, code, about, why]: [String; 4] = texts
                .try_into()
                .map_err(|_| ProgressFault::Field("a slot"))?;
            refusals.push(ProgressRefusal {
                day: get.u32(base + SLOT_DAY),
                stage,
                code,
                about,
                why,
            });
        }
        Ok(Self {
            state: get.u8(at::STATE),
            mode: get.u8(at::MODE),
            kind: get.u8(at::KIND),
            source: get.u8(at::SOURCE),
            by: get.u8(at::BY),
            layout: get.u8(at::LAYOUT),
            run: get.u64(at::RUN),
            pid: get.u32(at::PID),
            workers: get.u32(at::WORKERS),
            from: get.u32(at::FROM),
            to: get.u32(at::TO),
            day_now: get.u32(at::DAY_NOW),
            day_index: get.u32(at::DAY_INDEX),
            day_total: get.u32(at::DAY_TOTAL),
            files: counter(0),
            files_skipped: counter(1),
            files_refused: counter(2),
            rows: counter(3),
            seconds_written: counter(4),
            refusals_total: counter(5),
            days_imported: counter(6),
            days_with_refusals: counter(7),
            days_incomplete: counter(8),
            days_all_refused: counter(9),
            days_skipped: counter(10),
            days_closed: counter(11),
            days_not_held: counter(12),
            days_refused: counter(13),
            elapsed_s: counter(14),
            started: counter(15),
            updated: counter(16),
            clock_held: counter(17),
            window_days: counter(18),
            window_s: counter(19),
            log_dropped: counter(20),
            log_failures: counter(21),
            refusals,
        })
    }

    /// The record at `path`, read twice at most: a bad CRC is a torn read
    /// during a write, and is read once more before it is said (design
    /// §5.7).
    ///
    /// # Errors
    ///
    /// The file's read error in words, or the second read's
    /// [`ProgressFault`].
    pub fn read(path: &Path) -> Result<Self, String> {
        let once = || std::fs::read(path).map_err(|why| format!("{}: {why}", path.display()));
        match Self::decode(&once()?) {
            Err(ProgressFault::Crc) => Self::decode(&once()?).map_err(|fault| fault.to_string()),
            other => other.map_err(|fault| fault.to_string()),
        }
    }
}

/// `day` as `yyyymmdd`.
fn ymd(day: Day) -> u32 {
    u32::from(day.year()) * 10_000 + u32::from(day.month()) * 100 + u32::from(day.day())
}

/// Where a run of `kind` writes its progress on a store of `layout`
/// (design §5.7): `imports/gdfl.<kind>.progress` on v4,
/// `imports/gdfl.progress` on v3.
#[must_use]
pub fn progress_path(
    store: &Path,
    kind: ImportKind,
    layout: telemetry::run_dirs::GdflLayout,
) -> PathBuf {
    let name = match layout {
        telemetry::run_dirs::GdflLayout::V4 => format!("gdfl.{}.progress", kind.as_str()),
        telemetry::run_dirs::GdflLayout::V3 => "gdfl.progress".to_owned(),
    };
    store.join("imports").join(name)
}

/// The clock a [`ProgressFile`] stamps `updated` and `started` with: the
/// UNIX second, or a test's.
pub type WallClock = fn() -> u64;

/// The progress file of one run (D-4904), and the [`Observer`]
/// that keeps it: rewritten whole, with one positional write of 4 KiB at
/// offset 0, at every day that changed a day count and otherwise at most
/// once a second, and at the run's end. Elapsed time comes from
/// [`Instant`] and every difference is saturating, so a clock stepped
/// backwards can neither abort a release build nor make the record lie
/// about how long the run has taken (IA-GR-28).
///
/// A write that fails is kept, by name, in [`ProgressFile::fault`] and said
/// once in a `warn`; the run goes on (the progress file is a view of the
/// run, never its record: the journal is that).
#[derive(Debug)]
pub struct ProgressFile {
    path: PathBuf,
    clock: WallClock,
    state: Mutex<Kept>,
}

/// What a [`ProgressFile`] holds between writes.
#[derive(Debug)]
struct Kept {
    record: Progress,
    began: Instant,
    last_write: Option<Instant>,
    last_days: (usize, usize, usize, usize, usize),
    writes: u64,
    fault: Option<String>,
    file: Option<std::fs::File>,
}

impl ProgressFile {
    /// The progress file of the run `kind` of `plan` would write at `path`,
    /// by `by`, `pid`; nothing is written until the run's first day.
    #[must_use]
    pub fn new(path: PathBuf, plan: &Plan, kind: &KindPlan, layout: u8) -> Self {
        Self::with_clock(path, plan, kind, layout, now_unix_s)
    }

    /// [`ProgressFile::new`] stamped by `clock`.
    #[must_use]
    pub fn with_clock(
        path: PathBuf,
        plan: &Plan,
        kind: &KindPlan,
        layout: u8,
        clock: WallClock,
    ) -> Self {
        let request = &plan.request;
        let record = Progress {
            state: RunState::Running.code(),
            mode: request.mode.code(),
            kind: kind_code(kind.kind),
            source: kind.source.code(),
            by: request.by.code(),
            layout,
            pid: std::process::id(),
            workers: u32::try_from(request.workers.get()).unwrap_or(u32::MAX),
            from: ymd(request.from),
            to: ymd(request.to),
            started: clock(),
            ..Progress::default()
        };
        Self {
            path,
            clock,
            state: Mutex::new(Kept {
                record,
                began: Instant::now(),
                last_write: None,
                last_days: (0, 0, 0, 0, 0),
                writes: 0,
                fault: None,
                file: None,
            }),
        }
    }

    /// The file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// How many records were written.
    #[must_use]
    pub fn writes(&self) -> u64 {
        self.kept().writes
    }

    /// The first write that failed, by name.
    #[must_use]
    pub fn fault(&self) -> Option<String> {
        self.kept().fault.clone()
    }

    /// The record as last kept.
    #[must_use]
    pub fn record(&self) -> Progress {
        self.kept().record.clone()
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Ends the record for a run refused before or outside the importer
    /// (its source unopened): state `refused`, the refusal in a slot.
    pub fn refused(&self, refusal: &RunRefusal) {
        let mut kept = self.kept();
        kept.record.state = RunState::Refused.code();
        kept.record.refusals.push(ProgressRefusal {
            day: 0,
            stage: "run".to_owned(),
            code: refusal.code().to_owned(),
            about: "source".to_owned(),
            why: cut(&refusal.to_string(), SLOT_WHY).to_owned(),
        });
        self.write(&mut kept);
    }

    /// Copies the report's counters into the record.
    fn copy(record: &mut Progress, report: &Report) {
        let n = |count: usize| u64::try_from(count).unwrap_or(u64::MAX);
        record.files = n(report.files);
        record.files_skipped = n(report.files_skipped);
        record.files_refused = n(report.files_refused);
        record.rows = n(report.rows);
        record.seconds_written = n(report.seconds_committed);
        record.refusals_total = n(report.failures_total);
        record.days_imported = n(report.days_imported);
        record.days_with_refusals = n(report.closed_with_refusals.len());
        record.days_incomplete = n(report.closed_incomplete.len());
        record.days_all_refused = n(report.closed_refused.len());
        record.days_skipped = n(report.days_skipped);
        record.days_closed = n(report.days_closed);
        record.days_not_held = n(report.days_missing.len());
        record.days_refused = n(report.days_refused);
        record.refusals = report
            .recent
            .iter()
            .map(|(cause, failure)| ProgressRefusal {
                day: failure.day.map_or(0, ymd),
                stage: cut(cause.stage.as_str(), SLOT_STAGE).to_owned(),
                code: cut(cause.code, SLOT_CODE).to_owned(),
                about: cut(&failure.instrument, SLOT_ABOUT).to_owned(),
                why: cut(&failure.why, SLOT_WHY).to_owned(),
            })
            .collect();
    }

    /// Writes the kept record: one positional write at offset 0.
    fn write(&self, kept: &mut Kept) {
        use std::os::unix::fs::FileExt as _;
        let elapsed = kept.began.elapsed().as_secs();
        kept.record.elapsed_s = elapsed;
        kept.record.window_s = elapsed;
        kept.record.updated = (self.clock)();
        if let Some(health) = telemetry::global().map(telemetry::Sink::health) {
            kept.record.clock_held = health.clock_held;
            kept.record.log_dropped = health.dropped;
            kept.record.log_failures = health.rotation_failures;
        }
        let bytes = kept.record.encode();
        if kept.file.is_none() {
            let opened = self
                .path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| {
                    use std::os::unix::fs::OpenOptionsExt as _;
                    // A LINK OR A PIPE AT THE NAME IS REFUSED, NEVER FOLLOWED
                    // OR WAITED ON (RA-317): the record is written only to a
                    // regular file of its own.
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(false)
                        .custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK)
                        .open(&self.path)
                })
                .and_then(|file| {
                    if file.metadata()?.is_file() {
                        Ok(file)
                    } else {
                        Err(std::io::Error::other("not a regular file"))
                    }
                });
            match opened {
                Ok(file) => kept.file = Some(file),
                Err(why) => return self.failed(kept, &why),
            }
        }
        let written = kept
            .file
            .as_ref()
            .map_or(Ok(()), |file| file.write_all_at(&bytes, 0));
        match written {
            Ok(()) => {
                kept.writes = kept.writes.saturating_add(1);
                kept.last_write = Some(Instant::now());
            }
            Err(why) => self.failed(kept, &why),
        }
    }

    /// Keeps the first failed write by name and says it once.
    fn failed(&self, kept: &mut Kept, why: &std::io::Error) {
        if kept.fault.is_none() {
            let words = format!("{}: {why}", self.path.display());
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(TARGET, "progress file not written")
                    .with("path", words.as_str()),
            );
            kept.fault = Some(words);
        }
    }

    /// Whether a second has passed since the last write (or none was made).
    fn due(kept: &Kept) -> bool {
        kept.last_write
            .is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(1))
    }
}

impl Observer for ProgressFile {
    fn begun(&self, _kind: ImportKind, run: u64) {
        let mut kept = self.kept();
        kept.record.run = run;
        self.write(&mut kept);
    }

    fn day_begun(&self, _kind: ImportKind, day: Day, (index, total): (usize, usize)) {
        let mut kept = self.kept();
        kept.record.day_now = ymd(day);
        kept.record.day_index = u32::try_from(index).unwrap_or(u32::MAX);
        kept.record.day_total = u32::try_from(total).unwrap_or(u32::MAX);
        if Self::due(&kept) {
            self.write(&mut kept);
        }
    }

    fn day_passed(&self, _kind: ImportKind, _day: Day, report: &Report) {
        let mut kept = self.kept();
        Self::copy(&mut kept.record, report);
        kept.record.window_days = u64::from(kept.record.day_index).saturating_add(1);
        let days = (
            report.days_imported,
            report.closed_with_refusals.len(),
            report.closed_incomplete.len(),
            report.closed_refused.len(),
            report.days_missing.len() + report.days_refused,
        );
        // A DAY THAT CLOSED IS WRITTEN AT ONCE; a day skipped as done (a
        // resume's thousands) waits for the second.
        if days != kept.last_days || Self::due(&kept) {
            kept.last_days = days;
            self.write(&mut kept);
        }
    }

    fn finished(&self, _kind: ImportKind, end: Result<&Report, &ImportRefusal>) {
        let mut kept = self.kept();
        let state = match end {
            Ok(report) => {
                Self::copy(&mut kept.record, report);
                RunState::Finished
            }
            Err(refusal) => {
                if let Some(done) = refusal.report() {
                    Self::copy(&mut kept.record, done);
                }
                kept.record.refusals.push(ProgressRefusal {
                    day: 0,
                    stage: "run".to_owned(),
                    code: cut(refusal.code(), SLOT_CODE).to_owned(),
                    about: "run".to_owned(),
                    why: cut(&refusal.to_string(), SLOT_WHY).to_owned(),
                });
                match refusal {
                    ImportRefusal::CensusFull { .. } | ImportRefusal::CensusRefused { .. } => {
                        RunState::Census
                    }
                    ImportRefusal::Durability { .. } => RunState::Durability,
                    _ => RunState::Refused,
                }
            }
        };
        kept.record.state = state.code();
        self.write(&mut kept);
    }
}

#[cfg(test)]
#[path = "gdfl_run_tests.rs"]
mod tests;
