//! The check's fixed-stride index, and the one reader a later stage asks:
//! [`SecondChecks::status_of`] answers "what did the tick-to-second check say
//! of this share on this day" in O(1) per lookup, and
//! [`SecondChecks::file_digest`] names the exact files it answered from, for a
//! run identity (D-5180, D-5184).
//!
//! # The file, `tick-to-second-<year>.v<N>.idx`
//!
//! Written once per version beside the version's CSV, staged, made durable
//! and linked to a name that did not exist (never replaced, never rewritten).
//! Every integer is little-endian.
//!
//! | Offset | Bytes | Field |
//! |---|---|---|
//! | 0 | 8 | magic `BRXT2SX1` |
//! | 8 | 4 | format version, 1 |
//! | 12 | 2 | the calendar year |
//! | 14 | 2 | zero |
//! | 16 | 4 | the year's first day, days since 1970-01-01 |
//! | 20 | 4 | days in the year (365 or 366) |
//! | 24 | 4 | symbols |
//! | 28 | 4 | the record stride, [`STRIDE`] |
//! | 32 | 4 | bytes of the symbol table |
//! | 36 | 4 | zero |
//! | 40 | 8 | the highest segment sequence this version folded |
//! | 48 | 32 | BLAKE3 of the version's CSV, byte for byte |
//! | 80 | 48 | zero |
//! | 128 | names | the symbols, sorted, each followed by `\n` |
//! | 128 + names | symbols x days x [`STRIDE`] | one record per (symbol, day), symbol-major |
//!
//! A record is the status code (1 byte), the source code (1 byte), two zero
//! bytes, then four `u32`: the stored seconds, the seconds compared for the
//! fill-worsening measure, of those the seconds whose rule-A high or low is
//! worse than the stored bar, and the row's line in the CSV (1 = the first
//! row after the header). An all-zero record is a cell no row filled:
//! [`NotChecked::NotChecked`].
//!
//! # Cost
//!
//! Opening reads the newest version's file once (O(its bytes)), checks every
//! header field, hashes it and maps each symbol to its row of records.
//! [`SecondChecks::status_of`] is then one hash probe for the symbol, one
//! subtraction for the day and one 20-byte read from memory: O(1), measured
//! by every run that writes a version (its timings file) and named in
//! `docs/06-limits.md` (L-T2S-09).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use pull::session::Day;

/// The first eight bytes of every index.
pub const MAGIC: [u8; 8] = *b"BRXT2SX1";

/// The one format version this build writes and reads. A change to any byte
/// is a new version, never a reinterpretation (`CLAUDE.md` §3 rule 8).
pub const FORMAT: u32 = 1;

/// Bytes of the header.
pub const HEADER_LEN: usize = 128;

/// Bytes of one record: the two codes, two zero bytes and four `u32`.
pub const STRIDE: usize = 20;

/// The record is exactly its fields: a stride that left no room for one of
/// the four `u32` would drop it on write and read zero back (the first
/// image of this format did, at 16 bytes, and lost every row number).
const _: () = assert!(STRIDE == 4 + 4 * 4);

/// What the check said of a share-day, as a code. The codes are this
/// format's and are never renumbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// The store equals rule T (trades only) in every field and the digest.
    ExactT,
    /// The store equals rule A (every row's price in high and low), on any
    /// day, a special session's included.
    ExactA,
    /// A special session (`pull::calendar`) whose store equals rule T. One
    /// that equals rule A is [`Self::ExactA`] in the index (its CSV row still
    /// says `special_session`, with `compare` `exact_A`).
    SpecialSession,
    /// The store differs from rule T; the CSV names the fields.
    Differs,
    /// The store holds rule T with the closing-auction prints folded in (bar
    /// definitions 2 and 3), which this build's importer restates.
    AuctionPrintsFolded,
    /// The source files the share, the store holds nothing that day.
    RawOnly,
    /// The store holds seconds the compared source does not file.
    StoreOnly,
    /// The day could not be compared; the CSV names why.
    ReadFailed,
}

impl Status {
    /// Every status, in code order (code 1 first).
    pub const ALL: [Self; 8] = [
        Self::ExactT,
        Self::ExactA,
        Self::SpecialSession,
        Self::Differs,
        Self::AuctionPrintsFolded,
        Self::RawOnly,
        Self::StoreOnly,
        Self::ReadFailed,
    ];

    /// The status's code, 1 to 8 (0 is a cell no row filled).
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::ExactT => 1,
            Self::ExactA => 2,
            Self::SpecialSession => 3,
            Self::Differs => 4,
            Self::AuctionPrintsFolded => 5,
            Self::RawOnly => 6,
            Self::StoreOnly => 7,
            Self::ReadFailed => 8,
        }
    }

    /// The status a code names.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The status a CSV `status` cell names: its word before any `:`.
    #[must_use]
    pub fn of_word(word: &str) -> Option<Self> {
        if word == super::FOLDED_STATUS {
            return Some(Self::AuctionPrintsFolded);
        }
        match word.split(':').next().unwrap_or(word) {
            "exact_T" => Some(Self::ExactT),
            "exact_A" => Some(Self::ExactA),
            "special_session" => Some(Self::SpecialSession),
            "differs" => Some(Self::Differs),
            "raw_only" => Some(Self::RawOnly),
            "store_only" => Some(Self::StoreOnly),
            "read_failed" => Some(Self::ReadFailed),
            _ => None,
        }
    }
}

/// What the row was compared against. The codes are this format's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The archive member the month's `.prv` record names, its name, size and
    /// CRC-32 checked against the record before it was read.
    ArchiveMember,
    /// The tick-store day file the month's `.prv` record names, its name and
    /// length checked against the record.
    TickstoreDay,
    /// No `.prv` record names the day's source, and no archive was given to
    /// compare with.
    Unrecorded,
    /// An archive the run was given, which is NOT the landing's recorded
    /// source: evidence, labelled as such.
    NotTheLandingSource,
}

impl Source {
    /// Every source, in code order (code 1 first).
    pub const ALL: [Self; 4] = [
        Self::ArchiveMember,
        Self::TickstoreDay,
        Self::Unrecorded,
        Self::NotTheLandingSource,
    ];

    /// The code, 1 to 4.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::ArchiveMember => 1,
            Self::TickstoreDay => 2,
            Self::Unrecorded => 3,
            Self::NotTheLandingSource => 4,
        }
    }

    /// The source a code names.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The CSV word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::ArchiveMember => "archive_member",
            Self::TickstoreDay => "tickstore_day",
            Self::Unrecorded => "unrecorded",
            Self::NotTheLandingSource => "archive_not_the_landing_source",
        }
    }

    /// The source a CSV word names.
    #[must_use]
    pub fn of_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.word() == word)
    }
}

/// One share-day's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondStatus {
    /// What the check said.
    pub status: Status,
    /// What it compared against.
    pub source: Source,
    /// Seconds the store holds that day.
    pub store_seconds: u32,
    /// Seconds compared for the fill-worsening measure: stored seconds that
    /// rule A also has.
    pub fill_compared: u32,
    /// Of those, seconds whose rule-A high is above, or rule-A low below, the
    /// stored bar's: how often a fill priced at the second's extreme would be
    /// worse under rule A than under the stored rule-T bar.
    pub fill_worse: u32,
    /// The row's line in the version's CSV (1 = the first row).
    pub row: u32,
}

impl SecondStatus {
    /// Whether the stored seconds of the day are the landing source's rule-T
    /// seconds bit for bit: exact (or a special session compared exact)
    /// against the source the `.prv` record names. Anything else is a reason
    /// a fill must not trust the day without saying so.
    #[must_use]
    pub const fn trusted(&self) -> bool {
        matches!(self.status, Status::ExactT | Status::SpecialSession)
            && matches!(self.source, Source::ArchiveMember | Source::TickstoreDay)
    }

    /// The record's [`STRIDE`] bytes.
    #[must_use]
    pub fn image(&self) -> [u8; STRIDE] {
        let mut out = [0_u8; STRIDE];
        let fields = [
            self.store_seconds,
            self.fill_compared,
            self.fill_worse,
            self.row,
        ];
        if let Some(head) = out.get_mut(..2) {
            head.copy_from_slice(&[self.status.code(), self.source.code()]);
        }
        for (slot, value) in out
            .get_mut(4..)
            .unwrap_or_default()
            .chunks_exact_mut(4)
            .zip(fields)
        {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// A record's bytes, `None` for an all-zero (unfilled) record.
    ///
    /// # Errors
    ///
    /// A status or source code this format does not define, or nonzero
    /// padding, named.
    pub fn decode(bytes: &[u8]) -> Result<Option<Self>, String> {
        let [status, source, pad0, pad1, rest @ ..] = bytes else {
            return Err(format!("a record is {} bytes, not {STRIDE}", bytes.len()));
        };
        if rest.len() != STRIDE - 4 {
            return Err(format!("a record is {} bytes, not {STRIDE}", bytes.len()));
        }
        if bytes.iter().all(|b| *b == 0) {
            return Ok(None);
        }
        if (*pad0, *pad1) != (0, 0) {
            return Err("a record's padding is not zero".to_owned());
        }
        let word = |at: usize| {
            rest.get(at..at + 4)
                .and_then(|w| <[u8; 4]>::try_from(w).ok())
                .map_or(0, u32::from_le_bytes)
        };
        Ok(Some(Self {
            status: Status::from_code(*status)
                .ok_or_else(|| format!("status code {status} is not one this format defines"))?,
            source: Source::from_code(*source)
                .ok_or_else(|| format!("source code {source} is not one this format defines"))?,
            store_seconds: word(0),
            fill_compared: word(4),
            fill_worse: word(8),
            row: word(12),
        }))
    }
}

/// Why [`SecondChecks::status_of`] has no answer. Each is a reason, never a
/// default status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotChecked {
    /// No version of the day's year exists in the directory.
    NoVersion {
        /// The year.
        year: u16,
    },
    /// The year's newest version names no such share.
    SymbolUnknown {
        /// The year.
        year: u16,
    },
    /// The share is in the index and no row covers the day.
    NotChecked {
        /// The version the answer came from.
        version: u32,
    },
}

impl std::fmt::Display for NotChecked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoVersion { year } => write!(f, "no tick-to-second version of {year}"),
            Self::SymbolUnknown { year } => {
                write!(
                    f,
                    "the newest tick-to-second version of {year} names no such share"
                )
            }
            Self::NotChecked { version } => {
                write!(f, "version {version} holds no row for that share-day")
            }
        }
    }
}

/// One year's newest index, held in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearIndex {
    year: u16,
    version: u32,
    first_day: u32,
    days: u32,
    symbols: HashMap<String, u32>,
    names: Vec<String>,
    records: Vec<u8>,
    segments_through: u64,
    csv_digest: [u8; 32],
    digest: [u8; 32],
    path: PathBuf,
}

/// The cells of one index to write: `(symbol, day, status)`.
pub type Cell<'a> = (&'a str, Day, SecondStatus);

/// The index image of `year`, from every filled cell. Symbols are those the
/// cells name, sorted; a later cell for the same share-day replaces an
/// earlier one (the caller hands them in CSV order, where a share-day's
/// landing row follows its extra row only when it has one).
///
/// # Errors
///
/// A day outside `year`, a symbol holding a newline, or more symbols or
/// days than the format counts, named.
pub fn encode(
    year: u16,
    cells: &[Cell<'_>],
    segments_through: u64,
    csv_digest: [u8; 32],
) -> Result<Vec<u8>, String> {
    let (first, days) = year_span(year)?;
    let mut names: Vec<&str> = cells.iter().map(|(s, _, _)| *s).collect();
    names.sort_unstable();
    names.dedup();
    if let Some(bad) = names.iter().find(|n| n.contains('\n') || n.is_empty()) {
        return Err(format!("{bad:?} cannot be an index symbol"));
    }
    let table: String = names.iter().flat_map(|n| [*n, "\n"]).collect();
    let n_symbols = u32::try_from(names.len()).map_err(|_| "too many symbols".to_owned())?;
    let names_len = u32::try_from(table.len()).map_err(|_| "symbol table too long".to_owned())?;
    let cells_len = names
        .len()
        .checked_mul(usize::try_from(days).map_err(|_| "days".to_owned())?)
        .and_then(|n| n.checked_mul(STRIDE))
        .ok_or("the index would be too large")?;
    let mut out = vec![0_u8; HEADER_LEN];
    let put = |out: &mut Vec<u8>, at: usize, bytes: &[u8]| {
        if let Some(slot) = out.get_mut(at..at + bytes.len()) {
            slot.copy_from_slice(bytes);
        }
    };
    put(&mut out, 0, &MAGIC);
    put(&mut out, 8, &FORMAT.to_le_bytes());
    put(&mut out, 12, &year.to_le_bytes());
    put(&mut out, 16, &first.to_le_bytes());
    put(&mut out, 20, &days.to_le_bytes());
    put(&mut out, 24, &n_symbols.to_le_bytes());
    put(
        &mut out,
        28,
        &u32::try_from(STRIDE).unwrap_or(0).to_le_bytes(),
    );
    put(&mut out, 32, &names_len.to_le_bytes());
    put(&mut out, 40, &segments_through.to_le_bytes());
    put(&mut out, 48, &csv_digest);
    out.extend_from_slice(table.as_bytes());
    let base = out.len();
    out.resize(base + cells_len, 0);
    for (symbol, day, status) in cells {
        let row = names
            .binary_search(symbol)
            .map_err(|_| format!("{symbol} is not in the table"))?;
        let at = day
            .days_from_epoch()
            .checked_sub(first)
            .filter(|d| *d < days)
            .ok_or_else(|| format!("{day} is not a day of {year}"))?;
        let offset =
            (row * usize::try_from(days).unwrap_or(0) + usize::try_from(at).unwrap_or(0)) * STRIDE;
        put(&mut out, base + offset, &status.image());
    }
    Ok(out)
}

/// The first day of `year` in days since the epoch, and its day count.
///
/// # Errors
///
/// A year `pull::session::Day` cannot name.
pub fn year_span(year: u16) -> Result<(u32, u32), String> {
    let first = Day::new(year, 1, 1).map_err(|w| w.to_string())?;
    let last = Day::new(year, 12, 31).map_err(|w| w.to_string())?;
    Ok((
        first.days_from_epoch(),
        last.days_from_epoch() - first.days_from_epoch() + 1,
    ))
}

/// A little-endian `u32` at `at`.
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .and_then(|w| <[u8; 4]>::try_from(w).ok())
        .map_or(0, u32::from_le_bytes)
}

impl YearIndex {
    /// The index in `bytes`, read from `path` as `version`.
    ///
    /// # Errors
    ///
    /// Every field that is not this format's, named: the magic, the version,
    /// the year's span, the stride, the table, the length, a record.
    pub fn decode(bytes: &[u8], path: &Path, version: u32) -> Result<Self, String> {
        let bad = |what: &str| format!("{}: {what}", path.display());
        if bytes.get(..8) != Some(&MAGIC[..]) {
            return Err(bad("not a tick-to-second index (magic)"));
        }
        if u32_at(bytes, 8) != FORMAT {
            return Err(bad("an index format this build does not read"));
        }
        let year = bytes
            .get(12..14)
            .and_then(|w| <[u8; 2]>::try_from(w).ok())
            .map_or(0, u16::from_le_bytes);
        let (first, days) = year_span(year).map_err(|w| bad(&w))?;
        if u32_at(bytes, 16) != first || u32_at(bytes, 20) != days {
            return Err(bad("the header's days are not its year's"));
        }
        if usize::try_from(u32_at(bytes, 28)).ok() != Some(STRIDE) {
            return Err(bad("the record stride is not this format's"));
        }
        let n_symbols = usize::try_from(u32_at(bytes, 24)).map_err(|_| bad("symbols"))?;
        let names_len = usize::try_from(u32_at(bytes, 32)).map_err(|_| bad("table"))?;
        let table = bytes
            .get(HEADER_LEN..HEADER_LEN + names_len)
            .and_then(|t| std::str::from_utf8(t).ok())
            .ok_or_else(|| bad("the symbol table is not text of its stated length"))?;
        let names: Vec<String> = table.split_terminator('\n').map(str::to_owned).collect();
        if names.len() != n_symbols || !names.windows(2).all(|w| w.first() < w.get(1)) {
            return Err(bad(
                "the symbol table is not its stated count of sorted names",
            ));
        }
        let records = bytes
            .get(HEADER_LEN + names_len..)
            .unwrap_or_default()
            .to_vec();
        let want = n_symbols
            .checked_mul(usize::try_from(days).unwrap_or(0))
            .and_then(|n| n.checked_mul(STRIDE));
        if Some(records.len()) != want {
            return Err(bad("the records are not symbols x days x stride bytes"));
        }
        for record in records.chunks_exact(STRIDE) {
            SecondStatus::decode(record).map_err(|w| bad(&w))?;
        }
        let segments_through = bytes
            .get(40..48)
            .and_then(|w| <[u8; 8]>::try_from(w).ok())
            .map_or(0, u64::from_le_bytes);
        let mut csv_digest = [0_u8; 32];
        if let Some(d) = bytes.get(48..80) {
            csv_digest.copy_from_slice(d);
        }
        let symbols = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), u32::try_from(i).unwrap_or(u32::MAX)))
            .collect();
        Ok(Self {
            year,
            version,
            first_day: first,
            days,
            symbols,
            names,
            records,
            segments_through,
            csv_digest,
            digest: brutex_core::blake3::hash(bytes),
            path: path.to_path_buf(),
        })
    }

    /// The year.
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// The version this index is.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// The highest segment sequence the version folded.
    #[must_use]
    pub const fn segments_through(&self) -> u64 {
        self.segments_through
    }

    /// BLAKE3 of the version's CSV, as the index records it.
    #[must_use]
    pub const fn csv_digest(&self) -> [u8; 32] {
        self.csv_digest
    }

    /// BLAKE3 of this index file, byte for byte.
    #[must_use]
    pub const fn file_digest(&self) -> [u8; 32] {
        self.digest
    }

    /// The path read.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The symbols, sorted.
    #[must_use]
    pub fn symbols(&self) -> &[String] {
        &self.names
    }

    /// The answer for `symbol` on `day`, which must be a day of this year:
    /// one hash probe, one subtraction, one [`STRIDE`]-byte read.
    ///
    /// # Errors
    ///
    /// [`NotChecked::SymbolUnknown`] or [`NotChecked::NotChecked`].
    pub fn status_of(&self, symbol: &str, day: Day) -> Result<SecondStatus, NotChecked> {
        let row = *self
            .symbols
            .get(symbol)
            .ok_or(NotChecked::SymbolUnknown { year: self.year })?;
        let unfilled = NotChecked::NotChecked {
            version: self.version,
        };
        let at = day
            .days_from_epoch()
            .checked_sub(self.first_day)
            .filter(|d| *d < self.days)
            .ok_or_else(|| unfilled.clone())?;
        let offset = (u64::from(row) * u64::from(self.days) + u64::from(at))
            * u64::try_from(STRIDE).unwrap_or(0);
        let start = usize::try_from(offset).map_err(|_| unfilled.clone())?;
        let record = self
            .records
            .get(start..start + STRIDE)
            .ok_or_else(|| unfilled.clone())?;
        SecondStatus::decode(record).ok().flatten().ok_or(unfilled)
    }
}

/// The newest version of every year an output directory holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecondChecks {
    years: BTreeMap<u16, YearIndex>,
}

/// The version numbers of `kind` files (`csv`, `idx`, ...) of `year` that
/// `dir` holds, from one listing of the directory.
///
/// # Errors
///
/// A directory that cannot be listed, named. A directory that does not
/// exist holds none.
pub fn versions_in(dir: &Path, year: u16, kind: &str) -> Result<Vec<u32>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    let prefix = format!("tick-to-second-{year}.v");
    let suffix = format!(".{kind}");
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if let Some(n) = name
            .strip_prefix(&prefix)
            .and_then(|r| r.strip_suffix(&suffix))
            .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|n| n.parse::<u32>().ok())
        {
            out.push(n);
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// The years `dir` holds an index of, from one listing.
///
/// # Errors
///
/// A directory that cannot be listed, named.
pub fn years_in(dir: &Path) -> Result<Vec<u16>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if let Some(year) = name
            .strip_prefix("tick-to-second-")
            .and_then(|r| r.split_once(".v"))
            .filter(|(_, rest)| rest.rsplit_once('.').is_some_and(|(_, ext)| ext == "idx"))
            .and_then(|(y, _)| y.parse::<u16>().ok())
        {
            out.push(year);
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// The whole of one file this check wrote (a version's CSV or index, a day
/// segment), read only if it is a regular file standing at its own name
/// (D-5189).
///
/// Every such file is made by a staged write and a hard link, so it is
/// always a regular file; anything else at the name is not one of them and
/// is refused by name, never followed or waited on. The open is
/// `store::open_flags::open_read_no_follow` (`O_NOFOLLOW | O_NONBLOCK`): a
/// symbolic link is refused by the host (a dangling one is named a link,
/// not a missing file), and a named pipe opens at once and is refused by its
/// `fstat` instead of holding the run for good (verify-DATA-2-r3 follow-up).
/// One open, one `fstat` and one read of the file.
///
/// # Errors
///
/// A link, a pipe, a directory or any other non-regular file, or the host's
/// refusal, each naming `path`.
pub fn read_regular(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut file = store::open_flags::open_read_no_follow(path).map_err(|why| {
        if why.raw_os_error() == Some(store::open_flags::ELOOP) {
            format!(
                "{} is a symbolic link (never followed): this check's files are regular files \
                 it wrote itself",
                path.display()
            )
        } else {
            format!("{}: {why}", path.display())
        }
    })?;
    let meta = file
        .metadata()
        .map_err(|why| format!("{}: {why}", path.display()))?;
    if !meta.is_file() {
        use std::os::unix::fs::FileTypeExt as _;
        let kind = meta.file_type();
        let what = if kind.is_fifo() {
            "a named pipe"
        } else if kind.is_dir() {
            "a directory"
        } else if kind.is_socket() {
            "a socket"
        } else if kind.is_block_device() || kind.is_char_device() {
            "a device"
        } else {
            "something else"
        };
        return Err(format!(
            "{} is {what}, not a regular file: this check's files are regular files it wrote \
             itself",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.read_to_end(&mut bytes)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(bytes)
}

/// The newest index of `year` in `dir`, `None` when it holds none.
///
/// # Errors
///
/// A directory that cannot be listed, an index that cannot be read (or is
/// not a regular file, [`read_regular`]), or one that is not this format's,
/// named.
pub fn newest(dir: &Path, year: u16) -> Result<Option<YearIndex>, String> {
    let Some(&version) = versions_in(dir, year, "idx")?.last() else {
        return Ok(None);
    };
    let path = dir.join(format!("tick-to-second-{year}.v{version:03}.idx"));
    let bytes = read_regular(&path)?;
    YearIndex::decode(&bytes, &path, version).map(Some)
}

impl SecondChecks {
    /// The newest version of every year `dir` holds.
    ///
    /// # Errors
    ///
    /// What [`newest`] refuses for any year.
    pub fn open(dir: &Path) -> Result<Self, String> {
        let mut years = BTreeMap::new();
        for year in years_in(dir)? {
            if let Some(index) = newest(dir, year)? {
                years.insert(year, index);
            }
        }
        Ok(Self { years })
    }

    /// The answer for `symbol` on `day`: the year's index is found in a map
    /// of at most one entry per calendar year, then [`YearIndex::status_of`].
    ///
    /// # Errors
    ///
    /// [`NotChecked::NoVersion`] for a year no version covers, and what
    /// [`YearIndex::status_of`] refuses.
    pub fn status_of(&self, symbol: &str, day: Day) -> Result<SecondStatus, NotChecked> {
        self.years
            .get(&day.year())
            .ok_or(NotChecked::NoVersion { year: day.year() })?
            .status_of(symbol, day)
    }

    /// BLAKE3 over each held year's number, version and index digest, in year
    /// order: one value a run identity can carry to name exactly the files
    /// every [`Self::status_of`] answer came from.
    #[must_use]
    pub fn file_digest(&self) -> [u8; 32] {
        let mut h = brutex_core::blake3::Hasher::new();
        for (year, index) in &self.years {
            h.update(&year.to_le_bytes());
            h.update(&index.version.to_le_bytes());
            h.update(&index.digest);
        }
        h.finalize()
    }

    /// The years held, each with its version.
    #[must_use]
    pub fn versions(&self) -> Vec<(u16, u32)> {
        self.years.iter().map(|(y, i)| (*y, i.version)).collect()
    }

    /// The index of `year`, when held.
    #[must_use]
    pub fn year(&self, year: u16) -> Option<&YearIndex> {
        self.years.get(&year)
    }
}
