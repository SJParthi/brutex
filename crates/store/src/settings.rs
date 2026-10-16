//! The operator's settings, kept in the store: one file per generation,
//! never rewritten (R3-05).
//!
//! # What lives here
//!
//! The operator's window, the instrument `/db` opens on, the rows every
//! picker always shows and the source locations. Before this module each of
//! them was a literal in a crate or a page, or an environment value of the
//! start script, so changing one meant a rebuild or a restart and nothing
//! recorded who changed it or why.
//!
//! # The bytes on disk
//!
//! ```text
//! <store>/settings/v1/<generation, 20 digits>.settings
//! ```
//!
//! Each file is the WHOLE settings at that generation, not a difference, so
//! the newest is one read. Text, one field per line:
//!
//! ```text
//! brutex-settings 1
//! generation 7
//! changed_at_micros 1760000000000000
//! changed_by operator
//! reason widen the window to October
//! changed window.to
//! set window.from 2018-01-01
//! set window.to 2026-10-31
//! crc32c 1a2b3c4d
//! ```
//!
//! The checksum is CRC-32C over every byte before its line. A file that
//! fails it, or any line of the grammar, is a FAULT the reader names and
//! skips — never read as settings, never silently replaced by a default.
//!
//! # Append-only, and what that buys
//!
//! A save writes generation `n + 1` to a private temporary name, syncs it,
//! and then HARD-LINKS it to its final name. A hard link refuses an existing
//! name, so two writers that both saw generation `n` cannot both land: the
//! second is refused as stale (`CLAUDE.md` §3 rule 8 — a version is never
//! mutated in place). A writer killed before the link leaves only a
//! temporary file no reader opens; one killed after it leaves a complete
//! generation. No file is ever torn at its final name.
//!
//! THE LINK IS THE COMMIT. Once it succeeds the save has landed and is
//! answered [`Saved`], never refused: another writer that read the new
//! generation may land the next one before this save reads back, and that is
//! said as [`Saved::superseded_by`], not as a failure.
//!
//! The format is `docs/02-store-format.md` §41; its choices are D-4969.
//!
//! The generations themselves are the audit: each records when, by whom,
//! why and which keys changed, and every earlier generation is still on
//! disk to compare against.
//!
//! # Cost
//!
//! [`Reader::refresh`] after the first is ONE `stat` of the next generation's
//! name when nothing changed: O(1), independent of how many generations
//! exist. The first refresh lists the directory once, O(generations) — a
//! count that grows only when the operator saves, named for
//! `docs/06-limits.md`. A refresh that finds new generations reads each once,
//! at most [`MAX_STEP`] of them, each at most [`MAX_FILE_BYTES`].

use std::fmt::{self, Write as _};
use std::io::{ErrorKind, Write as _};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// The directory under the store root.
pub const DIR: &str = "settings";

/// The file version this build writes and reads.
pub const VERSION: u16 = 1;

/// The first word of every settings file.
const MAGIC: &str = "brutex-settings";

/// The largest settings file a reader opens. A bigger one is a fault.
pub const MAX_FILE_BYTES: u64 = 64 * 1_024;

/// The most instruments `always.shown` may name.
pub const MAX_ALWAYS_SHOWN: usize = 64;

/// The longest source path accepted, in bytes.
pub const MAX_PATH_BYTES: usize = 1_024;

/// The longest reason or author accepted, in bytes.
pub const MAX_NOTE_BYTES: usize = 240;

/// The most new generations one refresh reads before it stops and says so.
pub const MAX_STEP: u64 = 4_096;

/// The most read faults a reader keeps, newest first.
pub const MAX_FAULTS: usize = 16;

/// One setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// The first day of the operator's window, `YYYY-MM-DD`.
    WindowFrom,
    /// The last day of the operator's window, `YYYY-MM-DD`.
    WindowTo,
    /// The feed `/db` opens on (a feed's wire word).
    OpenFeed,
    /// The instrument `/db` opens on (a store symbol).
    OpenInstrument,
    /// The segment `/db` opens on: `INDEX`, `CASH` or `FNO`.
    OpenSegment,
    /// The timeframe `/db` opens on: a store rung, or `finest`.
    OpenTimeframe,
    /// The order `/db` opens in: `newest` or `oldest`.
    OpenOrder,
    /// The instruments every picker always shows, comma-separated.
    AlwaysShown,
    /// Where the GDFL archive files are delivered: an absolute directory.
    SourceGdfl,
    /// Where the exchange masters are kept: an absolute directory.
    SourceMasters,
}

/// How many keys exist.
pub const KEYS: usize = 10;

impl Key {
    /// Every key, in the order a file lists them.
    pub const ALL: [Self; KEYS] = [
        Self::WindowFrom,
        Self::WindowTo,
        Self::OpenFeed,
        Self::OpenInstrument,
        Self::OpenSegment,
        Self::OpenTimeframe,
        Self::OpenOrder,
        Self::AlwaysShown,
        Self::SourceGdfl,
        Self::SourceMasters,
    ];

    /// The key's name in a file and on the wire.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WindowFrom => "window.from",
            Self::WindowTo => "window.to",
            Self::OpenFeed => "open.feed",
            Self::OpenInstrument => "open.instrument",
            Self::OpenSegment => "open.segment",
            Self::OpenTimeframe => "open.timeframe",
            Self::OpenOrder => "open.order",
            Self::AlwaysShown => "always.shown",
            Self::SourceGdfl => "source.gdfl",
            Self::SourceMasters => "source.masters",
        }
    }

    /// The key a name spells, or `None`. At most [`KEYS`] comparisons.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.name() == name)
    }

    /// The start script's environment value that stands in for this key
    /// while no generation sets it — the named fallback. Only a value the
    /// start script really exports is named.
    #[must_use]
    pub const fn env(self) -> Option<&'static str> {
        match self {
            Self::SourceMasters => Some("BRUTEX_MASTERS"),
            _ => None,
        }
    }

    /// The key's slot in [`Settings`].
    #[must_use]
    pub const fn slot(self) -> usize {
        match self {
            Self::WindowFrom => 0,
            Self::WindowTo => 1,
            Self::OpenFeed => 2,
            Self::OpenInstrument => 3,
            Self::OpenSegment => 4,
            Self::OpenTimeframe => 5,
            Self::OpenOrder => 6,
            Self::AlwaysShown => 7,
            Self::SourceGdfl => 8,
            Self::SourceMasters => 9,
        }
    }
}

/// The values one generation sets. A key it does not set is `None`, which a
/// caller answers from the environment fallback or its own default — and
/// says which.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    values: [Option<String>; KEYS],
}

impl Settings {
    /// The value of `key`, if set.
    #[must_use]
    pub fn get(&self, key: Key) -> Option<&str> {
        self.values.get(key.slot()).and_then(Option::as_deref)
    }

    fn put(&mut self, key: Key, value: Option<String>) {
        if let Some(slot) = self.values.get_mut(key.slot()) {
            *slot = value;
        }
    }

    /// Every key whose value differs between `self` and `other`.
    #[must_use]
    pub fn differs(&self, other: &Self) -> Vec<Key> {
        Key::ALL
            .into_iter()
            .filter(|key| self.get(*key) != other.get(*key))
            .collect()
    }
}

/// One saved generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    /// Its number, from 1.
    pub number: u64,
    /// The whole settings at this generation.
    pub settings: Settings,
    /// When it was saved, UNIX microseconds.
    pub changed_at_micros: i64,
    /// Who saved it.
    pub changed_by: String,
    /// Why, in the saver's words.
    pub reason: String,
    /// The keys this generation changed from the one before it.
    pub changed: Vec<Key>,
    /// The file it was read from.
    pub path: PathBuf,
}

/// A generation that could not be read, named rather than skipped silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The generation number the file's name claims.
    pub generation: u64,
    /// The file.
    pub path: PathBuf,
    /// What is wrong with it.
    pub why: String,
}

/// Why a change was not saved. Every refusal is decided BEFORE the hard link
/// that publishes a generation, so nothing is at a generation's name when
/// one is returned: once the link succeeds, [`save`] answers [`Saved`] and
/// says what its read-back found beside it (R3-05 repair 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A value, or the change as a whole, failed its check. `key` is the
    /// key at fault, `None` for the change as a whole.
    Invalid {
        /// The key at fault.
        key: Option<Key>,
        /// Why.
        why: String,
    },
    /// Another save landed after the generation the caller saw.
    Stale {
        /// The generation the caller saw.
        expected: u64,
        /// The highest generation on disk.
        newest: u64,
    },
    /// The change would save exactly the settings already in force.
    Unchanged,
    /// The disk refused: full, read-only, or another I/O error.
    Io {
        /// The path being written.
        path: PathBuf,
        /// The error's kind.
        kind: ErrorKind,
        /// The error's words.
        why: String,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid {
                key: Some(key),
                why,
            } => write!(f, "{}: {why}", key.name()),
            Self::Invalid { key: None, why } => f.write_str(why),
            Self::Stale { expected, newest } => write!(
                f,
                "stale: the change was made against generation {expected}, and generation \
                 {newest} is on disk; read the settings again and repeat the change"
            ),
            Self::Unchanged => f.write_str("nothing changed: every value is already in force"),
            Self::Io { path, kind, why } => {
                write!(f, "the disk refused {}: {kind:?}: {why}", path.display())
            }
        }
    }
}

/// One requested change.
#[derive(Debug, Clone, Default)]
pub struct Change {
    /// The generation the caller read before deciding: `0` when none existed.
    pub expected: u64,
    /// Values to set.
    pub set: Vec<(Key, String)>,
    /// Keys to unset, so their fallback is in force again.
    pub unset: Vec<Key>,
    /// Who is saving.
    pub changed_by: String,
    /// Why.
    pub reason: String,
    /// When, UNIX microseconds.
    pub at_micros: i64,
}

/// The settings directory of the store at `root`.
#[must_use]
pub fn dir(root: &Path) -> PathBuf {
    root.join(DIR).join(format!("v{VERSION}"))
}

/// The file of generation `number`.
#[must_use]
pub fn path_of(root: &Path, number: u64) -> PathBuf {
    dir(root).join(format!("{number:020}.settings"))
}

/// Every generation number named in the settings directory of `root`,
/// ascending; empty when the directory does not exist. O(names in the
/// directory) plus a sort: run when a reader first lists and by every
/// [`save`], never by a refresh that only steps (`docs/06-limits.md`
/// L-R305-6).
fn list_numbers(root: &Path) -> Result<Vec<u64>, Fault> {
    let dir = dir(root);
    let mut numbers: Vec<u64> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str().and_then(number_of))
            .collect(),
        Err(why) if why.kind() == ErrorKind::NotFound => Vec::new(),
        Err(why) => {
            return Err(Fault {
                generation: 0,
                path: dir,
                why: why.to_string(),
            });
        }
    };
    numbers.sort_unstable();
    Ok(numbers)
}

/// The numbers missing from `numbers` (ascending, unique) between 1 and the
/// highest, in words: the first [`MAX_FAULTS`] and the count. `None` when
/// they run unbroken. O(numbers).
fn holes_in(numbers: &[u64]) -> Option<String> {
    let mut first = Vec::new();
    let mut count = 0_u64;
    let mut expected = 1_u64;
    for &number in numbers.iter().filter(|&&number| number > 0) {
        if number > expected {
            let missing = number - expected;
            count = count.saturating_add(missing);
            let mut at = expected;
            while first.len() < MAX_FAULTS && at < number {
                first.push(at.to_string());
                at += 1;
            }
        }
        // `u64::MAX` HAS NO SUCCESSOR: nothing above it can be missing.
        let Some(next) = number.checked_add(1) else {
            break;
        };
        expected = next;
    }
    (count > 0).then(|| {
        format!(
            "{count} generation number(s) missing below the highest on disk ({}{}): a file \
             was removed, or one was written past the next number; a reader stepping \
             n + 1 cannot see past a hole, so the generations above it come into force \
             only when the directory is listed again (a restart, or a save)",
            first.join(", "),
            if count > first.len() as u64 {
                ", ..."
            } else {
                ""
            }
        )
    })
}

/// Why no generation can follow `u64::MAX`.
fn exhausted_why() -> String {
    format!(
        "generation numbers are exhausted: generation {} is on disk and no later \
         generation can be named, so no save can land; a file was written at that \
         name outside a save",
        u64::MAX
    )
}

/// The generation a file name spells, or `None` for any other name.
fn number_of(name: &str) -> Option<u64> {
    let digits = name.strip_suffix(".settings")?;
    if digits.len() != 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

// -- checking one value ------------------------------------------------------

/// Whether `text` is a real civil day `YYYY-MM-DD` between 1990 and 2100.
fn civil_day(text: &str) -> Result<(u16, u8, u8), String> {
    let bytes = text.as_bytes();
    let shape = bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, b)| {
            if at == 4 || at == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    let part = |from: usize, to: usize| text.get(from..to).and_then(|s| s.parse::<u16>().ok());
    let (Some(year), Some(month), Some(day)) = (part(0, 4), part(5, 7), part(8, 10)) else {
        return Err(format!("{text:?} is not a day written YYYY-MM-DD"));
    };
    if !shape {
        return Err(format!("{text:?} is not a day written YYYY-MM-DD"));
    }
    // A RANGE PATTERN, never `.contains(&` (gate 11 rule 7's ambiguous test).
    if !matches!(year, 1990..=2100) {
        return Err(format!("{text:?}: the year must be 1990 to 2100"));
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(format!("{text:?}: there is no month {month}")),
    };
    if day == 0 || day > last {
        return Err(format!("{text:?}: that month has no day {day}"));
    }
    let month = u8::try_from(month).map_err(|why| why.to_string())?;
    let day = u8::try_from(day).map_err(|why| why.to_string())?;
    Ok((year, month, day))
}

/// Whether `text` is a store symbol.
fn symbol(text: &str) -> Result<(), String> {
    brutex_core::symbol::Symbol::new(text)
        .map(|_| ())
        .map_err(|_| format!("{text:?} is not a symbol the store can name"))
}

/// Whether `text` is shaped like a folder a setting may name: absolute, no
/// `..`, at most [`MAX_PATH_BYTES`]. Says nothing of whether it exists.
fn path_shape(text: &str) -> Result<(), String> {
    if text.len() > MAX_PATH_BYTES {
        return Err(format!("the path is longer than {MAX_PATH_BYTES} bytes"));
    }
    let path = Path::new(text);
    if !path.is_absolute() {
        return Err(format!("{text:?} is not an absolute path"));
    }
    if path.components().any(|part| part == Component::ParentDir) {
        return Err(format!(
            "{text:?} climbs with '..'; name the directory itself"
        ));
    }
    Ok(())
}

/// Whether `text` names an absolute directory that exists now.
fn directory(text: &str) -> Result<(), String> {
    path_shape(text)?;
    match std::fs::metadata(Path::new(text)) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(format!("{text:?} is not a directory")),
        Err(why) => Err(format!("{text:?} cannot be opened: {why}")),
    }
}

/// Whether `text` is free of control characters, so it fits on one line.
fn one_line(text: &str) -> bool {
    !text.chars().any(char::is_control)
}

/// Check one value and return it as it will be stored.
///
/// # Errors
///
/// [`Refusal::Invalid`] naming `key` and why.
pub fn check(key: Key, value: &str) -> Result<String, Refusal> {
    let invalid = |why: String| Refusal::Invalid {
        key: Some(key),
        why,
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid(
            "an empty value; unset the key to restore its fallback".to_owned(),
        ));
    }
    if !one_line(value) {
        return Err(invalid("a control character or line break".to_owned()));
    }
    match key {
        Key::WindowFrom | Key::WindowTo => {
            civil_day(value).map_err(invalid)?;
        }
        Key::OpenFeed => {
            if value.len() > 16 || !value.bytes().all(|b| b.is_ascii_lowercase()) {
                return Err(invalid(format!("{value:?} is not a feed's wire word")));
            }
        }
        Key::OpenInstrument => symbol(value).map_err(invalid)?,
        Key::OpenSegment => {
            brutex_core::instrument::Segment::parse(value)
                .map_err(|_| invalid(format!("{value:?} is not INDEX, CASH or FNO")))?;
        }
        Key::OpenTimeframe => {
            if value != "finest"
                && !crate::path::Timeframe::KNOWN
                    .iter()
                    .any(|tf| tf.as_str() == value)
            {
                return Err(invalid(format!(
                    "{value:?} is not a store timeframe or `finest`"
                )));
            }
        }
        Key::OpenOrder => {
            if value != "newest" && value != "oldest" {
                return Err(invalid(format!("{value:?} is not `newest` or `oldest`")));
            }
        }
        Key::AlwaysShown => {
            // ONE HASH PROBE PER NAME, pre-sized to the ceiling the loop
            // refuses past (gate 11 rules 3 and 7); the list keeps the order.
            let mut seen: Vec<&str> = Vec::with_capacity(MAX_ALWAYS_SHOWN + 1);
            let mut named: std::collections::HashSet<&str> =
                std::collections::HashSet::with_capacity(MAX_ALWAYS_SHOWN + 1);
            for one in value.split(',').map(str::trim) {
                symbol(one).map_err(invalid)?;
                if !named.insert(one) {
                    return Err(invalid(format!("{one:?} is named twice")));
                }
                seen.push(one);
                if seen.len() > MAX_ALWAYS_SHOWN {
                    return Err(invalid(format!("more than {MAX_ALWAYS_SHOWN} instruments")));
                }
            }
            return Ok(seen.join(","));
        }
        Key::SourceGdfl | Key::SourceMasters => directory(value).map_err(invalid)?,
    }
    Ok(value.to_owned())
}

/// The checks that span keys: a window's first day is not after its last.
fn check_whole(settings: &Settings) -> Result<(), Refusal> {
    if let (Some(from), Some(to)) = (settings.get(Key::WindowFrom), settings.get(Key::WindowTo))
        && from > to
    {
        return Err(Refusal::Invalid {
            key: Some(Key::WindowTo),
            why: format!("the window would end ({to}) before it begins ({from})"),
        });
    }
    Ok(())
}

/// The check a value READ BACK from a generation must pass: [`check`]'s own,
/// and the value must be exactly as a save writes it. A source folder is
/// checked for its shape only: a folder that has moved since it was saved
/// is the route's to report, not a damaged generation. A file whose
/// checksum holds but whose value fails is a fault — a hand-edited or
/// foreign file is never read as settings.
fn check_stored(key: Key, value: &str) -> Result<(), String> {
    let canonical = match key {
        Key::SourceGdfl | Key::SourceMasters => path_shape(value).map(|()| value.to_owned()),
        _ => check(key, value).map_err(|refusal| refusal.to_string()),
    }?;
    if canonical != value {
        return Err(format!(
            "{}: {value:?} is not written as a save writes it ({canonical:?})",
            key.name()
        ));
    }
    Ok(())
}

// -- the file ----------------------------------------------------------------

/// The bytes of one generation's file.
fn encode(number: u64, settings: &Settings, change: &Change, changed: &[Key]) -> String {
    let mut text = format!(
        "{MAGIC} {VERSION}\ngeneration {number}\nchanged_at_micros {}\nchanged_by {}\nreason {}\nchanged {}\n",
        change.at_micros,
        change.changed_by,
        change.reason,
        changed
            .iter()
            .map(|key| key.name())
            .collect::<Vec<_>>()
            .join(","),
    );
    for key in Key::ALL {
        if let Some(value) = settings.get(key) {
            text.push_str("set ");
            text.push_str(key.name());
            text.push(' ');
            text.push_str(value);
            text.push('\n');
        }
    }
    let crc = crate::crc::crc32c(text.as_bytes());
    // Writing to a `String` cannot fail.
    let _ = writeln!(text, "crc32c {crc:08x}");
    text
}

/// One generation's file, read back; `Err` names what is wrong.
fn decode(number: u64, path: &Path, text: &str) -> Result<Generation, String> {
    let body_end = text
        .trim_end_matches('\n')
        .rfind('\n')
        .map(|at| at + 1)
        .ok_or("no checksum line")?;
    let (body, tail) = text.split_at(body_end);
    let stated = tail
        .trim_end()
        .strip_prefix("crc32c ")
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .ok_or("the last line is not `crc32c <8 hex>`")?;
    let actual = crate::crc::crc32c(body.as_bytes());
    if stated != actual {
        return Err(format!(
            "checksum {stated:08x} stated, {actual:08x} computed: the file is damaged"
        ));
    }
    let mut lines = body.lines();
    let mut field = |name: &str| -> Result<String, String> {
        let line = lines.next().ok_or_else(|| format!("no `{name}` line"))?;
        line.strip_prefix(name)
            .and_then(|rest| {
                rest.strip_prefix(' ')
                    .or(if rest.is_empty() { Some("") } else { None })
            })
            .map(str::to_owned)
            .ok_or_else(|| format!("expected `{name} …`, found {line:?}"))
    };
    let version = field(MAGIC)?;
    if version != VERSION.to_string() {
        return Err(format!(
            "version {version} — this build reads version {VERSION} only"
        ));
    }
    let stated_number: u64 = field("generation")?
        .parse()
        .map_err(|_| "the generation is not a number".to_owned())?;
    if stated_number != number {
        return Err(format!(
            "the file says generation {stated_number} under the name of {number}"
        ));
    }
    let changed_at_micros: i64 = field("changed_at_micros")?
        .parse()
        .map_err(|_| "changed_at_micros is not a number".to_owned())?;
    let changed_by = field("changed_by")?;
    let reason = field("reason")?;
    let changed_words = field("changed")?;
    let mut changed = Vec::new();
    for word in changed_words.split(',').filter(|w| !w.is_empty()) {
        changed.push(Key::of_name(word).ok_or_else(|| format!("unknown changed key {word:?}"))?);
    }
    let mut settings = Settings::default();
    for line in lines {
        let rest = line
            .strip_prefix("set ")
            .ok_or_else(|| format!("unexpected line {line:?}"))?;
        let (name, value) = rest
            .split_once(' ')
            .ok_or_else(|| format!("a `set` line with no value: {line:?}"))?;
        let key = Key::of_name(name).ok_or_else(|| format!("unknown key {name:?}"))?;
        if settings.get(key).is_some() {
            return Err(format!("{name} is set twice"));
        }
        check_stored(key, value)?;
        settings.put(key, Some(value.to_owned()));
    }
    check_whole(&settings).map_err(|refusal| refusal.to_string())?;
    Ok(Generation {
        number,
        settings,
        changed_at_micros,
        changed_by,
        reason,
        changed,
        path: path.to_path_buf(),
    })
}

/// Read generation `number`'s file.
///
/// THROUGH THE ONE READING DOOR, [`crate::safe_open::open_read_len`]
/// (D-4951; R3-05 repair 3, the second agent's blocking 2): never through a
/// final symbolic link, never waiting, never anything but a regular file.
/// Before, the name was `stat`ed with `metadata`, which FOLLOWS a link, and
/// then read whole with `fs::read`: a FIFO (length 0) passed the size cap and
/// the read waited for a writer forever, and a link to `/dev/zero` was read
/// without end — each while the server held its settings lock. Now each is a
/// named fault (`not a regular file: pipe`), and the read takes at most
/// [`MAX_FILE_BYTES`] + 1 bytes even from a file that grows after its
/// `fstat`.
fn read_one(root: &Path, number: u64) -> Result<Generation, Fault> {
    use std::io::Read as _;
    let path = path_of(root, number);
    let fault = |why: String| Fault {
        generation: number,
        path: path.clone(),
        why,
    };
    let (file, len) = crate::safe_open::open_read_len(&path).map_err(|refusal| {
        fault(match refusal.found() {
            Some(found) => format!(
                "not a regular file: {} ({}); nothing was read through it and nothing \
                 waited on it",
                found.word(),
                found.sentence()
            ),
            None => refusal.to_string(),
        })
    })?;
    if len > MAX_FILE_BYTES {
        return Err(fault(format!(
            "{len} bytes, more than the {MAX_FILE_BYTES} a settings file may hold"
        )));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| fault(why.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(fault(format!(
            "it grew past the {MAX_FILE_BYTES} bytes a settings file may hold while it was read"
        )));
    }
    let text = String::from_utf8(bytes).map_err(|_| fault("not UTF-8".to_owned()))?;
    decode(number, &path, &text).map_err(fault)
}

// -- reading -----------------------------------------------------------------

/// The settings of one store, kept between requests and refreshed by one
/// `stat`.
#[derive(Debug, Clone)]
pub struct Reader {
    root: PathBuf,
    listed: bool,
    highest: u64,
    newest: Option<Arc<Generation>>,
    faults: Vec<Fault>,
    stopped: Option<String>,
    last_cleared: Option<(PathBuf, u64)>,
    holes: Option<String>,
}

impl Reader {
    /// A reader of the store at `root` that has read nothing yet.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            listed: false,
            highest: 0,
            newest: None,
            faults: Vec::new(),
            stopped: None,
            last_cleared: None,
            holes: None,
        }
    }

    /// The generation numbers missing below the highest one this reader
    /// listed, in words — the first [`MAX_FAULTS`] and how many in all — or
    /// `None` when the numbers run unbroken from 1. A hole is a fault, never
    /// invisible: saves only ever land `n + 1`, so a hole means a file was
    /// removed, or one was planted past the next number, and a reader
    /// stepping `n + 1` cannot see past it (R3-05 repair 4, the second
    /// agent's follow-up 2). Found when the directory is LISTED — on a
    /// reader's first refresh, and by every [`save`], which lists before it
    /// lands so that a save never lands in a hole a restart would supersede.
    #[must_use]
    pub fn holes(&self) -> Option<&str> {
        self.holes.as_deref()
    }

    /// The temporary name the last [`save`] through this reader found a
    /// dead writer's leftover at, and that leftover's bytes, which were
    /// cleared before the save wrote its own; `None` when the name was free
    /// (the usual case: the name holds the process, a counter and the
    /// clock).
    #[must_use]
    pub fn last_cleared(&self) -> Option<(&Path, u64)> {
        self.last_cleared
            .as_ref()
            .map(|(path, bytes)| (path.as_path(), *bytes))
    }

    /// The store root this reader reads.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The highest generation number on disk, valid or not; `0` for none.
    #[must_use]
    pub const fn highest(&self) -> u64 {
        self.highest
    }

    /// The newest generation that read cleanly.
    #[must_use]
    pub fn newest(&self) -> Option<&Arc<Generation>> {
        self.newest.as_ref()
    }

    /// The generations that did not read, newest first, at most
    /// [`MAX_FAULTS`].
    #[must_use]
    pub fn faults(&self) -> &[Fault] {
        &self.faults
    }

    /// Why the last refresh stopped short, if it did.
    #[must_use]
    pub fn stopped(&self) -> Option<&str> {
        self.stopped.as_deref()
    }

    fn fault(&mut self, fault: Fault) {
        self.faults.insert(0, fault);
        self.faults.truncate(MAX_FAULTS);
    }

    /// `generation` is in force. Every fault OLDER than it is dropped: the
    /// faults are always the generations NEWER than the one in force that
    /// did not read, which is exactly what a reader that has never listed
    /// the directory finds (it reads newest first and stops at the first
    /// that reads). Before, a live reader kept a fault below the generation
    /// in force until restart, so whether `/feeds.json` refused the window
    /// as degraded depended on the process's history (R3-05 repair 3, the
    /// second agent's follow-up 1). An older damaged generation is still
    /// named by `history`, which reads each one again.
    fn take(&mut self, generation: Generation) {
        let number = generation.number;
        self.faults.retain(|fault| fault.generation > number);
        self.newest = Some(Arc::new(generation));
    }

    /// Read again every generation that did not read, newest first; the
    /// first that reads now is in force. A fault is RE-READ, never
    /// remembered: a reader that has never listed would read them again, so
    /// a live one does too, and a refusal the host has stopped making (a
    /// permission restored, a disk remounted) is not served as damage until
    /// restart. At most [`MAX_FAULTS`] reads of at most [`MAX_FILE_BYTES`]
    /// each, and only while a fault is on the record; none when the settings
    /// are clean.
    fn reread_faults(&mut self) {
        if self.faults.is_empty() {
            return;
        }
        let pending: Vec<u64> = self.faults.iter().map(|fault| fault.generation).collect();
        let mut again = Vec::with_capacity(pending.len());
        for number in pending {
            match read_one(&self.root, number) {
                Ok(generation) => {
                    self.newest = Some(Arc::new(generation));
                    break;
                }
                Err(fault) => again.push(fault),
            }
        }
        self.faults = again;
    }

    /// Bring the reader up to the newest generation on disk.
    ///
    /// The first call lists the directory once; every later call is one
    /// `stat` when nothing was saved. A generation that does not read is
    /// recorded in [`Self::faults`] and the newest one that does stays in
    /// force — loudly, never as if the damaged one were absent.
    ///
    /// # Errors
    ///
    /// The directory exists and cannot be listed.
    pub fn refresh(&mut self) -> Result<(), Fault> {
        self.stopped = None;
        if !self.listed {
            let numbers = list_numbers(&self.root)?;
            // A RE-LIST STARTS FROM NOTHING, as a reader that has never
            // listed does: what it finds replaces what was kept.
            self.faults.clear();
            self.newest = None;
            self.listed = true;
            self.highest = numbers.last().copied().unwrap_or(0);
            self.holes = holes_in(&numbers);
            // NEWEST FIRST: the first that reads is the one in force; every
            // newer one that did not is a fault on the record.
            for number in numbers.into_iter().rev() {
                match read_one(&self.root, number) {
                    Ok(generation) => {
                        self.take(generation);
                        break;
                    }
                    // PUSHED, NOT INSERTED AT THE FRONT: this walk goes
                    // newest first, so pushing keeps the record newest first
                    // as `faults` promises. Inserting at the front, as the
                    // step below does, left a listed record oldest first, and
                    // `degraded` named the oldest fault (R3-05 repair 3).
                    Err(fault) => self.faults.push(fault),
                }
                if self.faults.len() >= MAX_FAULTS {
                    self.stopped = Some(format!(
                        "{MAX_FAULTS} newest generations did not read; no older one was tried"
                    ));
                    break;
                }
            }
            return Ok(());
        }
        self.reread_faults();
        let mut stepped = 0_u64;
        // `symlink_metadata`, NEVER `exists` or `try_exists`. `exists`
        // answers `false` for a name it could not stat, so a directory that
        // stopped being readable looked like one where nothing was saved
        // since (R3-05 repair 2). `try_exists` FOLLOWS a link, so a dangling
        // link at the next name looked like nothing saved, and every later
        // save's hard link was refused at that name as stale, for ever
        // (R3-05 repair 3). Whatever is at the name is read, and a name that
        // is not a regular file is a named fault.
        loop {
            // CHECKED, NEVER SATURATED: a generation named `u64::MAX` has no
            // successor. Saturating made `u64::MAX + 1` the name `u64::MAX`
            // itself, which exists, so every refresh re-read that one file
            // 4,096 times, reported the false reason "4,096 new generations
            // read", and every save hard-linked onto an existing name and was
            // refused as stale for ever (R3-05 repair 4, the second agent's
            // follow-up 1). The exhaustion is named, and nothing loops.
            let Some(number) = self.highest.checked_add(1) else {
                self.stopped = Some(exhausted_why());
                break;
            };
            let next = path_of(&self.root, number);
            match std::fs::symlink_metadata(&next) {
                Ok(_) => {}
                Err(why) if why.kind() == ErrorKind::NotFound => break,
                Err(why) => {
                    let why = why.to_string();
                    self.stopped = Some(format!(
                        "generation {number} cannot be looked for ({}): {why}",
                        next.display()
                    ));
                    return Err(Fault {
                        generation: number,
                        path: next,
                        why,
                    });
                }
            }
            if stepped >= MAX_STEP {
                self.stopped = Some(format!(
                    "{MAX_STEP} new generations read in one refresh; the rest wait for the next"
                ));
                break;
            }
            stepped += 1;
            self.highest = number;
            match read_one(&self.root, number) {
                Ok(generation) => self.take(generation),
                Err(fault) => self.fault(fault),
            }
        }
        Ok(())
    }
}

/// The `limit` newest generations below and including `from`, newest first,
/// each read again from disk; a generation that does not read is its fault.
/// O(limit) reads.
#[must_use]
pub fn history(root: &Path, from: u64, limit: usize) -> Vec<Result<Generation, Fault>> {
    (1..=from)
        .rev()
        .take(limit)
        .map(|number| read_one(root, number))
        .collect()
}

// -- saving ------------------------------------------------------------------

/// A test's step between a save's hard link and its read-back: it is handed
/// the store root and the generation number just linked.
#[cfg(test)]
pub(crate) type AfterLink = Box<dyn FnOnce(&Path, u64)>;

#[cfg(test)]
thread_local! {
    /// A test's step between a save's hard link and its read-back, run once
    /// and only on the thread that set it: the window in which another
    /// writer can land the next generation, made deterministic.
    pub(crate) static AFTER_LINK: std::cell::RefCell<Option<AfterLink>> =
        const { std::cell::RefCell::new(None) };
}

/// A counter of this process's saves, in every temporary name. Two threads of
/// one process that read the clock in the same nanosecond still differ by it
/// — the race test found exactly that collision, and a writer that then
/// removed "its" temporary file removed the other's.
static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The refusal for an I/O error at `path`.
fn io_refusal(path: &Path, why: &std::io::Error) -> Refusal {
    Refusal::Io {
        path: path.to_path_buf(),
        kind: why.kind(),
        why: why.to_string(),
    }
}

/// The settings `change` leaves in force over `before`: the author and the
/// reason, every value, then the window as a whole, each checked.
fn apply(before: &Settings, change: &Change) -> Result<Settings, Refusal> {
    for (name, text) in [
        ("changed_by", &change.changed_by),
        ("reason", &change.reason),
    ] {
        if text.trim().is_empty() || text.len() > MAX_NOTE_BYTES || !one_line(text) {
            return Err(Refusal::Invalid {
                key: None,
                why: format!("{name} must be one line of 1 to {MAX_NOTE_BYTES} bytes"),
            });
        }
    }
    let mut after = before.clone();
    for key in &change.unset {
        after.put(*key, None);
    }
    for (key, value) in &change.set {
        if change.unset.contains(key) {
            return Err(Refusal::Invalid {
                key: Some(*key),
                why: "both set and unset in one change".to_owned(),
            });
        }
        after.put(*key, Some(check(*key, value)?));
    }
    check_whole(&after)?;
    Ok(after)
}

/// Write `bytes` to a new temporary file in `dir` named for generation
/// `number`, synced: the temporary's path, and the bytes of a dead writer's
/// leftover that had the same name and was cleared first, if one did.
///
/// The name is this writer's alone (the process, [`SEQUENCE`], the clock),
/// and it is born by [`crate::staging::create`], the one door that opens
/// nothing already there (gate 28, D-4922; R3-05 repair 3, the second
/// agent's blocking 1): a hard link planted at the name is refused
/// `staging_linked` with the file behind it untouched, and a link, FIFO or
/// directory `staging_not_regular`. Only this writer removes the name.
fn write_temp(dir: &Path, number: u64, bytes: &[u8]) -> Result<(PathBuf, Option<u64>), Refusal> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let temp = dir.join(format!(
        ".{number:020}.{}-{}-{nanos}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let staged = crate::staging::create(&temp).map_err(|refusal| Refusal::Io {
        path: temp.clone(),
        kind: match &refusal {
            crate::staging::StagingError::Io { source, .. } => source.kind(),
            _ => ErrorKind::AlreadyExists,
        },
        why: refusal.to_string(),
    })?;
    let mut file = staged.file;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(why) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(io_refusal(&temp, &why));
    }
    Ok((temp, staged.cleared))
}

/// A save that LANDED: its hard link succeeded, so generation
/// `generation.number` is on disk and nothing about it is a refusal.
///
/// What the read-back after the link found is said beside the success,
/// never instead of it (R3-05 repair 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The generation this save wrote, built from the bytes it wrote.
    pub generation: Arc<Generation>,
    /// The generation in force after the read-back: this one, a later one
    /// another writer landed, or (when this one did not read back) an older
    /// one; `None` when none reads.
    pub in_force: Option<u64>,
    /// A later generation another writer landed after this save's link and
    /// before its read-back, now in force; `None` when this save is in force
    /// or did not read back.
    pub superseded_by: Option<u64>,
    /// Why the read-back after the link could not confirm this generation:
    /// the directory could not be looked in, or the file did not read.
    /// `None` when it read back.
    pub read_back: Option<String>,
}

/// Check `change` against the generation in force, then save it as the next
/// generation — or refuse with nothing written.
///
/// `extra` is the caller's own check of the whole result (a feed the server
/// does not offer, say); it runs after every value's own check and before
/// any byte is written.
///
/// ONCE THE HARD LINK SUCCEEDS THE ANSWER IS [`Saved`], whatever the
/// read-back finds: a successor generation another writer landed in the
/// meantime is [`Saved::superseded_by`], and a read-back that faults or
/// cannot look is [`Saved::read_back`]. No refusal is ever returned for a
/// generation that is on disk.
///
/// # Errors
///
/// Every refusal is returned BEFORE the link, with nothing at a generation's
/// name: [`Refusal::Stale`] when a generation newer than `change.expected` is
/// on disk (including one written by a racing save that saw the same
/// generation); [`Refusal::Invalid`] for a value, the window, the author or
/// reason, or `extra`; [`Refusal::Unchanged`] for a change that changes
/// nothing; [`Refusal::Io`] when the disk refuses — nothing is then visible.
pub fn save(
    reader: &mut Reader,
    change: &Change,
    extra: impl Fn(&Settings) -> Result<(), Refusal>,
) -> Result<Saved, Refusal> {
    reader.last_cleared = None;
    // THE STORE'S FAULT, NOT THE CALLER'S: a settings directory that cannot
    // be listed (a FILE at its name, a permission) is `Io`, which the route
    // answers 500 `io`. It was `Invalid`, a 400 that told the caller their
    // change was wrong (R3-05 repair 3, the second agent's follow-up).
    reader.refresh().map_err(|fault| Refusal::Io {
        path: fault.path,
        kind: ErrorKind::Other,
        why: format!("the settings directory cannot be read: {}", fault.why),
    })?;
    if change.expected != reader.highest() {
        return Err(Refusal::Stale {
            expected: change.expected,
            newest: reader.highest(),
        });
    }
    // A SAVE LISTS BEFORE IT LANDS. A generation past a hole is invisible
    // to a reader stepping `n + 1`, and a save landing in the hole would be
    // superseded on the next restart, silently (R3-05 repair 4, the second
    // agent's follow-up 2). Found, the reader lists again — so it agrees
    // with a restart from now on, the hole named in `holes` — and the save
    // is refused as stale against the true highest. One listing per save:
    // O(generations), named in `docs/06-limits.md` L-R305-6.
    let listed = list_numbers(reader.root()).map_err(|fault| Refusal::Io {
        path: fault.path,
        kind: ErrorKind::Other,
        why: format!("the settings directory cannot be listed: {}", fault.why),
    })?;
    if listed.last().is_some_and(|&top| top > reader.highest()) {
        reader.listed = false;
        reader.refresh().map_err(|fault| Refusal::Io {
            path: fault.path,
            kind: ErrorKind::Other,
            why: format!("the settings directory cannot be read: {}", fault.why),
        })?;
        return Err(Refusal::Stale {
            expected: change.expected,
            newest: reader.highest(),
        });
    }
    reader.holes = holes_in(&listed);
    // NO NUMBER AFTER `u64::MAX`: refused by name, never a hard link onto
    // the existing name refused as stale for ever.
    let Some(number) = reader.highest().checked_add(1) else {
        return Err(Refusal::Io {
            path: path_of(reader.root(), u64::MAX),
            kind: ErrorKind::Other,
            why: exhausted_why(),
        });
    };
    let before = reader
        .newest()
        .map(|generation| generation.settings.clone())
        .unwrap_or_default();
    let after = apply(&before, change)?;
    extra(&after)?;
    let changed = before.differs(&after);
    if changed.is_empty() {
        return Err(Refusal::Unchanged);
    }
    let dir = dir(reader.root());
    std::fs::create_dir_all(&dir).map_err(|why| io_refusal(&dir, &why))?;
    let (temp, cleared) = write_temp(
        &dir,
        number,
        encode(number, &after, change, &changed).as_bytes(),
    )?;
    // A DEAD WRITER'S LEFTOVER CLEARED AT THIS NAME IS SAID, never dropped:
    // the caller reports it with the save (`Reader::last_cleared`).
    reader.last_cleared = cleared.map(|bytes| (temp.clone(), bytes));
    // A HARD LINK REFUSES A NAME THAT EXISTS: of two writers that both saw
    // generation `n`, exactly one lands `n + 1`.
    let target = path_of(reader.root(), number);
    let linked = std::fs::hard_link(&temp, &target);
    // THE TEMPORARY NAME IS NEVER READ; removing it is tidiness, and a
    // failure to remove it leaves a file no reader opens.
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => {}
        Err(why) if why.kind() == ErrorKind::AlreadyExists => {
            let _ = reader.refresh();
            return Err(Refusal::Stale {
                expected: change.expected,
                newest: reader.highest().max(number),
            });
        }
        Err(why) => return Err(io_refusal(&target, &why)),
    }
    Ok(landed(
        reader,
        &dir,
        Generation {
            number,
            settings: after,
            changed_at_micros: change.at_micros,
            changed_by: change.changed_by.clone(),
            reason: change.reason.clone(),
            changed,
            path: target,
        },
    ))
}

/// The answer to a save whose hard link SUCCEEDED: `generation` is on disk,
/// built here from the bytes the save wrote, and the answer is [`Saved`]
/// whatever the read-back finds.
fn landed(reader: &mut Reader, dir: &Path, generation: Generation) -> Saved {
    // THE NEW NAME IS DURABLE ONLY WHEN EVERY DIRECTORY ON ITS PATH IS:
    // `create_dir_all` may have made `settings/` and `settings/v<N>/` on the
    // first save, and their own entries live in their parents. Each of the
    // three directories is synced, newest name first (R3-05 repair 4, the
    // second agent's follow-up: the first generation's directories were never
    // synced, so a crash could lose the whole directory a save reported).
    // A sync the host refuses is tidiness lost, never the save: the
    // generation's own bytes were synced before it was linked.
    for synced in [Some(dir), dir.parent(), Some(reader.root())]
        .into_iter()
        .flatten()
    {
        if let Ok(handle) = std::fs::File::open(synced) {
            let _ = handle.sync_all();
        }
    }
    let number = generation.number;
    #[cfg(test)]
    if let Some(step) = AFTER_LINK.with(|slot| slot.borrow_mut().take()) {
        step(reader.root(), number);
    }
    // THE LINK LANDED THE SAVE. From here nothing is a refusal: generation
    // `number` is on disk under this writer's name, and an answer of
    // "refused" would tell the operator, and `/audit`, that a change in force
    // was never made (R3-05 repair 5, the second agent's blocking 1). Before,
    // the answer was taken from the read-back alone: a successor writer that
    // read `number` and landed `number + 1` between this link and the refresh
    // below made the reader step past this save, and the save was answered
    // `Invalid` "the reader did not take it" — 100 of 100 trials on the
    // operator's volume. The generation answered is built from the bytes this
    // save wrote, never from what a later read happens to find.
    let generation = Arc::new(generation);
    let target = &generation.path;
    let mut read_back = reader.refresh().err().map(|fault| {
        format!(
            "saved as {}; the reader could not look for it afterwards ({}): {}",
            target.display(),
            fault.path.display(),
            fault.why
        )
    });
    let in_force = reader.newest().map(|newest| newest.number);
    let superseded_by = in_force.filter(|&newest| newest > number);
    if read_back.is_none() && in_force.is_none_or(|newest| newest < number) {
        // WHY IT DID NOT READ is this generation's own file read once more,
        // never a search of the reader's fault list: one read, O(1).
        let why = match read_one(reader.root(), number) {
            Err(fault) => fault.why,
            Ok(_) => "it reads when read again, and the refresh did not step to it".to_owned(),
        };
        read_back = Some(format!(
            "saved as {}, and it did not read back, so the generation in force is {}: {why}",
            target.display(),
            in_force.map_or_else(|| "none".to_owned(), |newest| newest.to_string()),
        ));
    }
    Saved {
        generation,
        in_force,
        superseded_by,
        read_back,
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
