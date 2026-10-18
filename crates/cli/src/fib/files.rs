//! The FIB run files v1 under `results/fib-v1/<run-id>/` (FIB-8; D-6444):
//! one container for every FIB file kind, its fixed-stride records, and the
//! sealed exam-lock file. `docs/02-store-format.md` "FIB run files v1" is the
//! byte-level statement; this module is the one writer and the one reader.
//!
//! # The container
//!
//! | Part | Bytes |
//! |---|---|
//! | header | [`MAGIC`], format version, kind code, record version, `FIB_CUBE_VERSION`, record stride, records per block, record count, schema digest, run identity, scope (stock and month, month, partition, partition and month, or run), profile length, profile bytes, then blake3 of every header byte before it |
//! | body | `record count × stride` bytes; record `i` at `header + i × stride` |
//! | block table | one blake3 per block of `records per block` records |
//! | footer | record count, blake3 of the whole body, [`FOOTER_MAGIC`] |
//!
//! Every integer is little-endian. The block length is
//! [`vocab::fib_cube::FIB_FILE_BLOCK_BYTES`], a Representation entry of the
//! fixed-number ledger: a block holds that many bytes of whole records, and at
//! least one record.
//!
//! # Written once, never seen half written
//!
//! [`write_file`] makes its temporary through `store::staging::create` (the one
//! door for a temporary, CI gate 28), writes the header, body, block table and
//! footer, syncs, renames over the final name and syncs the directory. A
//! failure at any step removes the temporary, so no reader ever sees a half
//! file; a final name that already exists is refused (`already_sealed`), never
//! overwritten: a sealed file is never patched.
//!
//! # O(1) reads that never serve an unverified byte
//!
//! [`FibFile::open`] reads only the header (its digest checked) and the file's
//! length. [`FibFile::record`] finds record `i` at one multiply and add; the
//! first read in a block reads the whole block and its table entry and
//! verifies it, once per block per handle; a mismatch refuses that block by
//! name and it is never served (FM-F091). [`verify_whole`] still hashes the
//! whole body, every block and the footer.
//!
//! # The kinds
//!
//! [`Kind`] is append-only: a kind's code is its position, and a new kind is a
//! new last variant, so every v1 file stays decodable (`v1_kinds_decode_after_block`).
//! Where FIB-8 owns a kind's record layout it ships the [`Record`] type
//! ([`CloseRecord`], [`ReferenceCell`], [`CandidateRecord`], [`DeepRecord`],
//! [`MarksWord`], [`ExamLock`]); a kind whose record belongs to a later
//! package (FIB-2's rank, FIB-7's downside readings, the results of FIB-9 to
//! FIB-14) is a container with the stride its header declares, fixed for the
//! file, until its owner declares the layout.
//!
//! Measured, every lookup and record operation of this module: `crates/cli/benches/fib.rs` (FIB-8's rows, C-F8-01 to C-F8-10).

use std::fs::{self, File};
use std::io::{SeekFrom, Write as _};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use brutex_core::blake3::{Hasher, hash};
use vocab::fib_cube::{Coords, FIB_CUBE_VERSION, FIB_FILE_BLOCK_BYTES, ROW_COUNT};

use super::authorities::SeriesId;

/// The header's first bytes.
pub const MAGIC: &[u8] = b"BTXFIBF1";
/// The footer's last bytes.
pub const FOOTER_MAGIC: &[u8] = b"BTXFIBE1";
/// The FIB run files version these readers and writers speak.
pub const FORMAT_VERSION: u32 = 1;
/// The bytes of one digest.
pub const DIGEST: usize = vocab::fib_cube::DIGEST_BYTES;

// ---------------------------------------------------------------- error --

/// Why a FIB file could not be written or read, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileError {
    /// The refusal's word.
    pub word: &'static str,
    /// What was refused, and where.
    pub detail: String,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.word, self.detail)
    }
}

fn refuse<T>(word: &'static str, detail: String) -> Result<T, FileError> {
    Err(FileError { word, detail })
}

fn io(path: &Path, action: &str, why: &std::io::Error) -> FileError {
    FileError {
        word: "io",
        detail: format!("{action} {}: {why}", path.display()),
    }
}

// ---------------------------------------------------------------- bytes --

/// Little-endian appends.
#[derive(Debug, Default)]
pub struct Put(pub Vec<u8>);

impl Put {
    /// One byte.
    pub fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    /// Two bytes.
    pub fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    /// Four bytes.
    pub fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    /// Eight bytes.
    pub fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    /// Eight bytes.
    pub fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    /// Sixteen bytes.
    pub fn i128(&mut self, value: i128) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    /// The bytes as they are.
    pub fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(value);
    }
}

/// Little-endian reads that refuse a short slice by name.
#[derive(Debug)]
pub struct Take<'a> {
    rest: &'a [u8],
}

impl<'a> Take<'a> {
    /// Reads from the start of `bytes`.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    /// The next `n` bytes.
    ///
    /// # Errors
    ///
    /// `short` when fewer remain.
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.rest.len() < n {
            return Err(format!(
                "short: {} bytes wanted, {} left",
                n,
                self.rest.len()
            ));
        }
        let (head, rest) = self.rest.split_at(n);
        self.rest = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let mut out = [0; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    /// One byte.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn u8(&mut self) -> Result<u8, String> {
        self.array().map(u8::from_le_bytes)
    }
    /// Two bytes.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn u16(&mut self) -> Result<u16, String> {
        self.array().map(u16::from_le_bytes)
    }
    /// Four bytes.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn u32(&mut self) -> Result<u32, String> {
        self.array().map(u32::from_le_bytes)
    }
    /// Eight bytes.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn u64(&mut self) -> Result<u64, String> {
        self.array().map(u64::from_le_bytes)
    }
    /// Eight bytes.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn i64(&mut self) -> Result<i64, String> {
        self.array().map(i64::from_le_bytes)
    }
    /// Sixteen bytes.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn i128(&mut self) -> Result<i128, String> {
        self.array().map(i128::from_le_bytes)
    }
    /// A digest.
    ///
    /// # Errors
    ///
    /// `short`.
    pub fn digest(&mut self) -> Result<[u8; DIGEST], String> {
        self.array()
    }
    /// Bytes left unread.
    #[must_use]
    pub const fn left(&self) -> usize {
        self.rest.len()
    }
}

/// A `bool` read back from its byte, refusing any other value.
///
/// # Errors
///
/// `not_a_flag` for a byte that is neither 0 nor 1.
pub fn flag(byte: u8) -> Result<bool, String> {
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(format!("not_a_flag: byte {other}")),
    }
}

// ---------------------------------------------------------------- kinds --

/// Every FIB file kind. APPEND-ONLY: a kind's code is its position plus one,
/// so a new kind is a new last variant and no code is ever reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Per stock-month: the minute closes 09:15..15:09 and each day's
    /// previous close ([`CloseRecord`]).
    Closes,
    /// Per month: the mover rank (FIB-2's layout).
    Rank,
    /// Per month: the reference table, `[minute][index slot]` ([`ReferenceCell`]).
    Reference,
    /// Per stock-month: stage A's candidates ([`CandidateRecord`] v1).
    Candidates,
    /// Per partition and month: one sealed rows segment.
    Rows,
    /// Per partition: the survivors (FIB-6/7's layout).
    Survivors,
    /// Per stock-month: the marks bitset over candidate ordinals ([`MarksWord`]).
    Marks,
    /// Per stock-month: stage DEEP's outcomes ([`DeepRecord`] v1).
    Deep,
    /// Per run: the winners (FIB-14).
    Winners,
    /// Per run: the keepers (FIB-14).
    Keepers,
    /// Per run: the walk-forward steps (FIB-13).
    Steps,
    /// Per run: the diagnostics (FIB-14).
    Diagnostics,
    /// Per run: the luck receipt (FIB-7).
    LuckReceipt,
    /// Per run: the census (FIB-9).
    Census,
    /// Per run: the sealed exam lock ([`ExamLock`], FIB-1 R6).
    ExamLock,
    /// Per partition: the side passers (FM-F057, FIB-7's layout).
    SidePassers,
    /// Per run: the pairs (FM-F057, FIB-7's layout).
    Pairs,
    /// Per stock-month: the own-past histogram checkpoint (FM-F013, FIB-2's layout).
    HistogramCheckpoint,
    /// Per stock-month: the seconds summary (FM-F107, FIB-9's layout).
    SecondsSummary,
    /// Per partition: `downside v1` readings (FIB-7's layout).
    Downside,
    /// Per partition and month: `downside refs v1` (FIB-7's layout).
    DownsideRefs,
    /// Per run: the `GateRefusal` census v2 (FIB-7's layout).
    GateRefusalCensus,
}

/// What one file of a kind covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// One stock-month.
    StockMonth,
    /// One month of the pool.
    Month,
    /// One partition.
    Partition,
    /// One partition's month.
    PartitionMonth,
    /// The whole run.
    Run,
}

impl ScopeKind {
    /// Every scope kind, in byte order.
    pub const ALL: &[Self] = &[
        Self::StockMonth,
        Self::Month,
        Self::Partition,
        Self::PartitionMonth,
        Self::Run,
    ];
}

impl Kind {
    /// Every kind, in code order.
    pub const ALL: &[Self] = &[
        Self::Closes,
        Self::Rank,
        Self::Reference,
        Self::Candidates,
        Self::Rows,
        Self::Survivors,
        Self::Marks,
        Self::Deep,
        Self::Winners,
        Self::Keepers,
        Self::Steps,
        Self::Diagnostics,
        Self::LuckReceipt,
        Self::Census,
        Self::ExamLock,
        Self::SidePassers,
        Self::Pairs,
        Self::HistogramCheckpoint,
        Self::SecondsSummary,
        Self::Downside,
        Self::DownsideRefs,
        Self::GateRefusalCensus,
    ];

    /// The kind's code: its position plus one. Never renumbered.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16 + 1
    }

    /// The kind of `code`.
    #[must_use]
    pub fn of_code(code: u16) -> Option<Self> {
        Self::ALL.get(usize::from(code.checked_sub(1)?)).copied()
    }

    /// The kind's name, as `docs/02-store-format.md` writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Closes => "closes",
            Self::Rank => "rank",
            Self::Reference => "reference",
            Self::Candidates => "candidates",
            Self::Rows => "rows",
            Self::Survivors => "survivors",
            Self::Marks => "marks",
            Self::Deep => "deep",
            Self::Winners => "winners",
            Self::Keepers => "keepers",
            Self::Steps => "steps",
            Self::Diagnostics => "diagnostics",
            Self::LuckReceipt => "luck receipt",
            Self::Census => "census",
            Self::ExamLock => "exam lock",
            Self::SidePassers => "side passers",
            Self::Pairs => "pairs",
            Self::HistogramCheckpoint => "histogram checkpoint",
            Self::SecondsSummary => "seconds summary",
            Self::Downside => "downside v1",
            Self::DownsideRefs => "downside refs v1",
            Self::GateRefusalCensus => "gate refusal census v2",
        }
    }

    /// What one file of the kind covers.
    #[must_use]
    pub const fn scope(self) -> ScopeKind {
        match self {
            Self::Closes
            | Self::Candidates
            | Self::Marks
            | Self::Deep
            | Self::HistogramCheckpoint
            | Self::SecondsSummary => ScopeKind::StockMonth,
            Self::Rank | Self::Reference => ScopeKind::Month,
            Self::Survivors | Self::SidePassers | Self::Downside => ScopeKind::Partition,
            Self::Rows | Self::DownsideRefs => ScopeKind::PartitionMonth,
            Self::Winners
            | Self::Keepers
            | Self::Steps
            | Self::Diagnostics
            | Self::LuckReceipt
            | Self::Census
            | Self::ExamLock
            | Self::Pairs
            | Self::GateRefusalCensus => ScopeKind::Run,
        }
    }

    /// The package that owns the kind's record layout.
    #[must_use]
    pub const fn owner(self) -> &'static str {
        match self {
            Self::Closes
            | Self::Reference
            | Self::Candidates
            | Self::Rows
            | Self::Marks
            | Self::Deep
            | Self::ExamLock => "FIB-8",
            Self::Rank | Self::HistogramCheckpoint => "FIB-2",
            Self::Survivors
            | Self::LuckReceipt
            | Self::SidePassers
            | Self::Pairs
            | Self::Downside
            | Self::DownsideRefs
            | Self::GateRefusalCensus => "FIB-7",
            Self::Census | Self::SecondsSummary => "FIB-9",
            Self::Steps => "FIB-13",
            Self::Winners | Self::Keepers | Self::Diagnostics => "FIB-14",
        }
    }

    /// The record stride FIB-8's layout fixes for the kind, or `None` where the
    /// owner's layout is not declared yet and the header's stride rules.
    #[must_use]
    pub fn stride(self) -> Option<u32> {
        match self {
            Self::Closes => Some(CloseRecord::stride()),
            Self::Reference => Some(ReferenceCell::stride()),
            Self::Candidates => Some(CandidateRecord::stride()),
            Self::Rows => Some(RowRecord::stride()),
            Self::Marks => Some(MarksWord::stride()),
            Self::Deep => Some(DeepRecord::stride()),
            Self::ExamLock => Some(ExamLock::stride()),
            _ => None,
        }
    }
}

/// What one file covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// One stock-month.
    StockMonth {
        /// The series.
        series: SeriesId,
        /// The year.
        year: u16,
        /// The month.
        month: u8,
    },
    /// One month of the pool.
    Month {
        /// The year.
        year: u16,
        /// The month.
        month: u8,
    },
    /// One partition.
    Partition {
        /// The partition.
        partition: u32,
    },
    /// One partition's month.
    PartitionMonth {
        /// The partition.
        partition: u32,
        /// The year.
        year: u16,
        /// The month.
        month: u8,
    },
    /// The whole run.
    Run,
}

impl Scope {
    /// The scope's kind.
    #[must_use]
    pub const fn kind(self) -> ScopeKind {
        match self {
            Self::StockMonth { .. } => ScopeKind::StockMonth,
            Self::Month { .. } => ScopeKind::Month,
            Self::Partition { .. } => ScopeKind::Partition,
            Self::PartitionMonth { .. } => ScopeKind::PartitionMonth,
            Self::Run => ScopeKind::Run,
        }
    }

    fn put(self, out: &mut Put) {
        let (series, year, month, partition) = match self {
            Self::StockMonth {
                series,
                year,
                month,
            } => (series.0, year, month, 0),
            Self::Month { year, month } => (0, year, month, 0),
            Self::Partition { partition } => (0, 0, 0, partition),
            Self::PartitionMonth {
                partition,
                year,
                month,
            } => (0, year, month, partition),
            Self::Run => (0, 0, 0, 0),
        };
        out.u8(self.kind() as u8);
        out.u32(series);
        out.u16(year);
        out.u8(month);
        out.u32(partition);
    }

    fn take(input: &mut Take<'_>) -> Result<Self, String> {
        let kind = input.u8()?;
        let (series, year, month, partition) =
            (input.u32()?, input.u16()?, input.u8()?, input.u32()?);
        let scope = match ScopeKind::ALL.get(usize::from(kind)) {
            Some(ScopeKind::StockMonth) => Self::StockMonth {
                series: SeriesId(series),
                year,
                month,
            },
            Some(ScopeKind::Month) => Self::Month { year, month },
            Some(ScopeKind::Partition) => Self::Partition { partition },
            Some(ScopeKind::PartitionMonth) => Self::PartitionMonth {
                partition,
                year,
                month,
            },
            Some(ScopeKind::Run) => Self::Run,
            None => return Err(format!("unknown_scope: byte {kind}")),
        };
        // A field a scope does not carry must be zero: one meaning per byte.
        let mut stated = Put::default();
        stated.u8(kind);
        stated.u32(series);
        stated.u16(year);
        stated.u8(month);
        stated.u32(partition);
        let mut again = Put::default();
        scope.put(&mut again);
        if again.0 == stated.0 {
            Ok(scope)
        } else {
            Err(format!(
                "scope_fields: {scope:?} carries a field it does not own"
            ))
        }
    }
}

// --------------------------------------------------------------- header --

/// What a writer states about a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// The kind.
    pub kind: Kind,
    /// The record layout's version for the kind.
    pub record_version: u16,
    /// Bytes per record.
    pub stride: u32,
    /// The run's schema digest.
    pub schema_digest: [u8; DIGEST],
    /// The run identity.
    pub run_identity: [u8; DIGEST],
    /// What the file covers.
    pub scope: Scope,
    /// The profile bytes the records were computed under.
    pub profile: Vec<u8>,
}

/// What a reader found in a header beyond what the writer stated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    /// The stated header.
    pub header: Header,
    /// Records in the body.
    pub records: u64,
    /// Records per checksum block.
    pub block_records: u64,
    /// The header's length in bytes, its digest included.
    pub header_len: u64,
}

/// Records per block for `stride`: as many whole records as fit in
/// [`FIB_FILE_BLOCK_BYTES`], and at least one.
#[must_use]
pub fn block_records(stride: u32) -> u64 {
    (FIB_FILE_BLOCK_BYTES / u64::from(stride.max(1))).max(1)
}

/// The header's bytes, its digest last.
///
/// # Errors
///
/// `profile_too_long` for a profile past `u32::MAX` bytes.
pub fn header_bytes(header: &Header, records: u64) -> Result<Vec<u8>, FileError> {
    let profile_len = u32::try_from(header.profile.len()).or_else(|_| {
        refuse(
            "profile_too_long",
            format!("{} profile bytes", header.profile.len()),
        )
    })?;
    let mut out = Put::default();
    out.bytes(MAGIC);
    out.u32(FORMAT_VERSION);
    out.u16(header.kind.code());
    out.u16(header.record_version);
    out.u32(FIB_CUBE_VERSION);
    out.u32(header.stride);
    out.u64(block_records(header.stride));
    out.u64(records);
    out.bytes(&header.schema_digest);
    out.bytes(&header.run_identity);
    header.scope.put(&mut out);
    out.u32(profile_len);
    out.bytes(&header.profile);
    let digest = hash(&out.0);
    out.bytes(&digest);
    Ok(out.0)
}

/// The header's fixed part, up to and including the profile length: the
/// header of an empty profile, less its digest.
#[must_use]
pub fn fixed_header_len() -> usize {
    let probe = Header {
        kind: Kind::ExamLock,
        record_version: 0,
        stride: 1,
        schema_digest: [0; DIGEST],
        run_identity: [0; DIGEST],
        scope: Scope::Run,
        profile: Vec::new(),
    };
    header_bytes(&probe, 0).map_or(0, |bytes| bytes.len() - DIGEST)
}

/// The footer's length.
#[must_use]
pub fn footer_len() -> usize {
    size_of::<u64>() + DIGEST + FOOTER_MAGIC.len()
}

/// Reads a header from its bytes: the fixed part, then the profile, then the
/// digest, which must match.
///
/// # Errors
///
/// `bad_magic`, `bad_version`, `unknown_kind`, `header_short`,
/// `header_digest` and every scope refusal, by name.
pub fn parse_header(fixed: &[u8], rest: &[u8]) -> Result<Sealed, FileError> {
    let short = |why: String| FileError {
        word: "header_short",
        detail: why,
    };
    let mut take = Take::new(fixed);
    if take.bytes(MAGIC.len()).map_err(short)? != MAGIC {
        return refuse(
            "bad_magic",
            "the file does not start with BTXFIBF1".to_owned(),
        );
    }
    let format = take.u32().map_err(short)?;
    if format != FORMAT_VERSION {
        return refuse(
            "bad_version",
            format!("format version {format}; this reader speaks {FORMAT_VERSION}"),
        );
    }
    let code = take.u16().map_err(short)?;
    let kind = Kind::of_code(code).ok_or_else(|| FileError {
        word: "unknown_kind",
        detail: format!("kind code {code}"),
    })?;
    let record_version = take.u16().map_err(short)?;
    let cube = take.u32().map_err(short)?;
    if cube != FIB_CUBE_VERSION {
        return refuse(
            "bad_version",
            format!("FIB cube version {cube}; this build is {FIB_CUBE_VERSION}"),
        );
    }
    let stride = take.u32().map_err(short)?;
    let blocks = take.u64().map_err(short)?;
    let records = take.u64().map_err(short)?;
    let schema_digest = take.digest().map_err(short)?;
    let run_identity = take.digest().map_err(short)?;
    let scope = Scope::take(&mut take).map_err(|why| FileError {
        word: "bad_scope",
        detail: why,
    })?;
    let profile_len = take.u32().map_err(short)?;
    let mut tail = Take::new(rest);
    let profile = tail
        .bytes(usize::try_from(profile_len).unwrap_or(usize::MAX))
        .map_err(short)?
        .to_vec();
    tail.digest().map_err(short)?;
    let read = rest.len() - tail.left();
    let header = Header {
        kind,
        record_version,
        stride,
        schema_digest,
        run_identity,
        scope,
        profile,
    };
    check(&header)?;
    if blocks != block_records(stride) {
        return refuse(
            "bad_block_length",
            format!(
                "{blocks} records per block; stride {stride} gives {}",
                block_records(stride)
            ),
        );
    }
    // The canonical bytes of what was read, its digest recomputed, must be the
    // bytes read: a changed byte anywhere in the header fails here.
    let expected = header_bytes(&header, records)?;
    let actual = fixed.iter().chain(rest.get(..read).unwrap_or_default());
    if !expected.iter().eq(actual) {
        return refuse(
            "header_digest",
            format!("the {} header does not match its digest", kind.name()),
        );
    }
    Ok(Sealed {
        header,
        records,
        block_records: blocks,
        header_len: u64::try_from(expected.len()).unwrap_or(u64::MAX),
    })
}

/// Refuses a header whose stride, record version or scope the kind does not
/// take.
fn check(header: &Header) -> Result<(), FileError> {
    let kind = header.kind;
    if header.stride == 0 {
        return refuse("stride_zero", format!("a {} file of stride 0", kind.name()));
    }
    if let Some(stride) = kind.stride()
        && stride != header.stride
    {
        return refuse(
            "stride_mismatch",
            format!(
                "a {} record is {stride} bytes, not {}",
                kind.name(),
                header.stride
            ),
        );
    }
    if header.scope.kind() != kind.scope() {
        return refuse(
            "scope_mismatch",
            format!(
                "a {} file covers {:?}, not {:?}",
                kind.name(),
                kind.scope(),
                header.scope.kind()
            ),
        );
    }
    Ok(())
}

// --------------------------------------------------------------- writer --

/// What a sealed write produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// The final path.
    pub path: PathBuf,
    /// Records written.
    pub records: u64,
    /// blake3 of the whole body.
    pub body_digest: [u8; DIGEST],
    /// blake3 of every byte of the file.
    pub file_digest: [u8; DIGEST],
    /// Bytes of a dead writer's temporary the staging door cleared first.
    pub cleared: Option<u64>,
}

/// Writes one sealed FIB file at `path` from `records`, each exactly
/// `header.stride` bytes, the first failure ending the write.
///
/// # Errors
///
/// `already_sealed` when `path` exists; every header refusal; `record_length`
/// for a record of another length; the producer's own refusal as `producer`;
/// a staging refusal or a host refusal as `io`. On every error the temporary
/// is removed and `path` is never created.
pub fn write_file<'a, I>(path: &Path, header: &Header, records: I) -> Result<Written, FileError>
where
    I: IntoIterator<Item = Result<&'a [u8], String>>,
{
    check(header)?;
    if fs::symlink_metadata(path).is_ok() {
        return refuse(
            "already_sealed",
            format!(
                "{} exists; a sealed file is never overwritten",
                path.display()
            ),
        );
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    let staged = store::staging::create(&temporary).map_err(|why| FileError {
        word: "io",
        detail: why.to_string(),
    })?;
    let cleared = staged.cleared;
    let mut file = staged.file;
    let mut buffered = std::io::BufWriter::new(&mut file);
    let filled = fill(&mut buffered, &temporary, header, records);
    // `fill` flushed the buffer, so the parts hold no unwritten byte.
    drop(buffered.into_parts());
    // The bytes are made durable and published in one step: a host refusal
    // of the sync or the rename leaves the temporary to be removed below,
    // exactly as a refusal while filling does.
    let sealed = filled.and_then(|filled| {
        file.sync_all()
            .and_then(|()| fs::rename(&temporary, path))
            .map(|()| filled)
            .map_err(|why| io(path, "sync and publish", &why))
    });
    match sealed {
        Ok(filled) => {
            sync_directory(path)?;
            Ok(Written {
                path: path.to_path_buf(),
                records: filled.records,
                body_digest: filled.body_digest,
                file_digest: filled.file_digest,
                cleared,
            })
        }
        Err(why) => {
            drop(file);
            let _ = fs::remove_file(&temporary);
            Err(why)
        }
    }
}

/// What [`fill`] wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Filled {
    records: u64,
    body_digest: [u8; DIGEST],
    file_digest: [u8; DIGEST],
}

/// Syncs the directory holding `path`, so the rename itself is durable.
fn sync_directory(path: &Path) -> Result<(), FileError> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(directory)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| io(directory, "sync the directory", &why))
}

/// Writes the whole file into `sink`: a placeholder header, the body, the
/// block table and the footer, then the real header over the placeholder,
/// then a flush. The file digest is blake3 over the real header, the body
/// digest, the block table and the footer — every byte of the file bound
/// without reading it back.
fn fill<'a, W, I>(
    sink: &mut W,
    temporary: &Path,
    header: &Header,
    records: I,
) -> Result<Filled, FileError>
where
    W: std::io::Write + std::io::Seek,
    I: IntoIterator<Item = Result<&'a [u8], String>>,
{
    let failed = |action: &'static str| move |why: std::io::Error| io(temporary, action, &why);
    let placeholder = header_bytes(header, 0)?;
    sink.write_all(&placeholder)
        .map_err(failed("write the header of"))?;
    let stride = usize::try_from(header.stride).unwrap_or(usize::MAX);
    let per_block = block_records(header.stride);
    let mut body = Hasher::new();
    let mut block = Hasher::new();
    let mut in_block = 0_u64;
    let mut table = Vec::new();
    let mut count = 0_u64;
    for record in records {
        let record = record.map_err(|why| FileError {
            word: "producer",
            detail: format!("record {count}: {why}"),
        })?;
        if record.len() != stride {
            return refuse(
                "record_length",
                format!(
                    "record {count} is {} bytes; the stride is {stride}",
                    record.len()
                ),
            );
        }
        sink.write_all(record)
            .map_err(failed("write a record of"))?;
        body.update(record);
        block.update(record);
        in_block += 1;
        count += 1;
        if in_block == per_block {
            table.extend_from_slice(&block.finalize());
            block = Hasher::new();
            in_block = 0;
        }
    }
    if in_block > 0 {
        table.extend_from_slice(&block.finalize());
    }
    let body_digest = body.finalize();
    let mut footer = Put::default();
    footer.u64(count);
    footer.bytes(&body_digest);
    footer.bytes(FOOTER_MAGIC);
    sink.write_all(&table)
        .map_err(failed("write the block table of"))?;
    sink.write_all(&footer.0)
        .map_err(failed("write the footer of"))?;
    let real = header_bytes(header, count)?;
    sink.seek(SeekFrom::Start(0))
        .map_err(failed("seek to the header of"))?;
    sink.write_all(&real)
        .map_err(failed("rewrite the header of"))?;
    sink.flush().map_err(failed("flush"))?;
    let mut whole = Hasher::new();
    whole.update(&real);
    whole.update(&body_digest);
    whole.update(&table);
    whole.update(&footer.0);
    Ok(Filled {
        records: count,
        body_digest,
        file_digest: whole.finalize(),
    })
}

// --------------------------------------------------------------- reader --

/// An open sealed FIB file: its header read and checked, its blocks verified
/// on first touch.
#[derive(Debug)]
pub struct FibFile {
    file: File,
    path: PathBuf,
    sealed: Sealed,
    verified: Vec<u64>,
    cache: Option<(u64, Vec<u8>)>,
    bytes_read: u64,
}

impl FibFile {
    /// Opens `path`, reading only its header and its length.
    ///
    /// # Errors
    ///
    /// Every [`parse_header`] refusal; `truncated` when the length is not the
    /// header, body, block table and footer the header implies; `io`.
    pub fn open(path: &Path) -> Result<Self, FileError> {
        let file = File::open(path).map_err(|why| io(path, "open", &why))?;
        let length = file.metadata().map_err(|why| io(path, "stat", &why))?.len();
        let fixed_len = fixed_header_len();
        let mut fixed = vec![0; fixed_len];
        file.read_exact_at(&mut fixed, 0).map_err(|why| FileError {
            word: "truncated",
            detail: format!("{} holds no whole header: {why}", path.display()),
        })?;
        let mut take = Take::new(&fixed);
        let profile_len = take
            .bytes(fixed_len - size_of::<u32>())
            .and_then(|_| take.u32())
            .map_err(|why| FileError {
                word: "header_short",
                detail: why,
            })?;
        let rest_len = usize::try_from(profile_len)
            .unwrap_or(usize::MAX)
            .saturating_add(DIGEST);
        let fixed_u64 = u64::try_from(fixed_len).unwrap_or(u64::MAX);
        if u64::try_from(rest_len).unwrap_or(u64::MAX) > length.saturating_sub(fixed_u64) {
            return refuse(
                "truncated",
                format!("{} ends inside its header", path.display()),
            );
        }
        let mut rest = vec![0; rest_len];
        file.read_exact_at(&mut rest, fixed_u64)
            .map_err(|why| io(path, "read the header of", &why))?;
        let sealed = parse_header(&fixed, &rest)?;
        let expected = expected_length(&sealed);
        if expected != Some(length) {
            return refuse(
                "truncated",
                format!(
                    "{} is {length} bytes; its header implies {}",
                    path.display(),
                    expected.map_or_else(|| "more than a u64".to_owned(), |n| n.to_string())
                ),
            );
        }
        let blocks = blocks_of(&sealed);
        let words = usize::try_from(blocks.div_ceil(u64::from(u64::BITS))).unwrap_or(usize::MAX);
        let header_len = sealed.header_len;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            sealed,
            verified: vec![0; words],
            cache: None,
            bytes_read: header_len,
        })
    }

    /// The header.
    #[must_use]
    pub const fn sealed(&self) -> &Sealed {
        &self.sealed
    }

    /// Records in the body.
    #[must_use]
    pub const fn records(&self) -> u64 {
        self.sealed.records
    }

    /// Bytes read from the file through this handle: the header, then each
    /// block verified and each record read.
    #[must_use]
    pub const fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Record `index`'s bytes. O(1): one multiply and add finds it; the first
    /// read in its block also reads and verifies the block.
    ///
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    ///
    /// # Errors
    ///
    /// `record_out_of_range`; `block_corrupt` naming the block when its bytes
    /// do not match its table entry (never served); `io`.
    pub fn record(&mut self, index: u64) -> Result<Vec<u8>, FileError> {
        if index >= self.sealed.records {
            return refuse(
                "record_out_of_range",
                format!(
                    "record {index} of {} in {}",
                    self.sealed.records,
                    self.path.display()
                ),
            );
        }
        let stride = u64::from(self.sealed.header.stride);
        let block = index / self.sealed.block_records;
        let (word, bit) = bit_of(block);
        let seen = self.verified.get(word).is_some_and(|bits| bits & bit != 0);
        if !seen {
            let bytes = self.verify_block(block)?;
            if let Some(bits) = self.verified.get_mut(word) {
                *bits |= bit;
            }
            self.cache = Some((block, bytes));
        }
        let within = index % self.sealed.block_records;
        let start = usize::try_from(within * stride).unwrap_or(usize::MAX);
        let length = usize::try_from(stride).unwrap_or(usize::MAX);
        if let Some((cached, bytes)) = &self.cache
            && *cached == block
            && let Some(record) = bytes.get(start..start.saturating_add(length))
        {
            return Ok(record.to_vec());
        }
        let mut record = vec![0; length];
        let at = self.sealed.header_len + index * stride;
        self.file
            .read_exact_at(&mut record, at)
            .map_err(|why| io(&self.path, "read a record of", &why))?;
        self.bytes_read += stride;
        Ok(record)
    }

    /// Reads block `block` and its table entry, and refuses it unless they match.
    fn verify_block(&mut self, block: u64) -> Result<Vec<u8>, FileError> {
        let sealed = &self.sealed;
        let stride = u64::from(sealed.header.stride);
        let first = block * sealed.block_records;
        let count = sealed.block_records.min(sealed.records - first);
        let mut bytes = vec![0; usize::try_from(count * stride).unwrap_or(usize::MAX)];
        let at = sealed.header_len + first * stride;
        self.file
            .read_exact_at(&mut bytes, at)
            .map_err(|why| io(&self.path, "read a block of", &why))?;
        let digest_u64 = u64::try_from(DIGEST).unwrap_or(u64::MAX);
        let table_at = sealed.header_len + sealed.records * stride + block * digest_u64;
        let mut stated = [0; DIGEST];
        self.file
            .read_exact_at(&mut stated, table_at)
            .map_err(|why| io(&self.path, "read the block table of", &why))?;
        self.bytes_read += count * stride + digest_u64;
        if hash(&bytes) != stated {
            return refuse(
                "block_corrupt",
                format!(
                    "block {block} of {} does not match its checksum; it is refused, never served",
                    self.path.display()
                ),
            );
        }
        Ok(bytes)
    }

    /// Record `index`, decoded as `R`.
    ///
    /// # Errors
    ///
    /// `wrong_kind` when the file is not `R`'s kind and version; every
    /// [`Self::record`] refusal; `record_decode`.
    pub fn read<R: Record>(&mut self, index: u64) -> Result<R, FileError> {
        let header = &self.sealed.header;
        if header.kind != R::KIND || header.record_version != R::VERSION {
            return refuse(
                "wrong_kind",
                format!(
                    "{} holds {} v{}, not {} v{}",
                    self.path.display(),
                    header.kind.name(),
                    header.record_version,
                    R::KIND.name(),
                    R::VERSION
                ),
            );
        }
        let bytes = self.record(index)?;
        R::decode(&bytes).map_err(|why| FileError {
            word: "record_decode",
            detail: format!("record {index}: {why}"),
        })
    }
}

/// The word and bit of `block` in a verified bitset.
fn bit_of(block: u64) -> (usize, u64) {
    let bits = u64::from(u64::BITS);
    (
        usize::try_from(block / bits).unwrap_or(usize::MAX),
        1_u64 << (block % bits),
    )
}

fn blocks_of(sealed: &Sealed) -> u64 {
    sealed.records.div_ceil(sealed.block_records)
}

/// The file length a header implies, or `None` past a `u64`.
fn expected_length(sealed: &Sealed) -> Option<u64> {
    let digest = u64::try_from(DIGEST).ok()?;
    sealed
        .records
        .checked_mul(u64::from(sealed.header.stride))?
        .checked_add(blocks_of(sealed).checked_mul(digest)?)?
        .checked_add(sealed.header_len)?
        .checked_add(u64::try_from(footer_len()).ok()?)
}

/// What a whole-file verification found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// The header.
    pub sealed: Sealed,
    /// blake3 of the whole body, equal to the footer's.
    pub body_digest: [u8; DIGEST],
}

/// Hashes the whole body, every block and the footer of `path` (`fib verify`).
///
/// # Errors
///
/// Every [`FibFile::open`] and block refusal; `footer` when the footer's count
/// or magic disagrees; `body_digest` when the body's digest does not match.
pub fn verify_whole(path: &Path) -> Result<Verified, FileError> {
    let mut handle = FibFile::open(path)?;
    let blocks = blocks_of(&handle.sealed);
    let mut body = Hasher::new();
    for block in 0..blocks {
        body.update(&handle.verify_block(block)?);
    }
    let sealed = handle.sealed.clone();
    let digest_u64 = u64::try_from(DIGEST).unwrap_or(u64::MAX);
    let footer_at =
        sealed.header_len + sealed.records * u64::from(sealed.header.stride) + blocks * digest_u64;
    let mut footer = vec![0; footer_len()];
    handle
        .file
        .read_exact_at(&mut footer, footer_at)
        .map_err(|why| io(path, "read the footer of", &why))?;
    let mut take = Take::new(&footer);
    let parsed = (take.u64(), take.digest(), take.bytes(FOOTER_MAGIC.len()));
    let (Ok(count), Ok(stated), Ok(magic)) = parsed else {
        return refuse("footer", format!("{} has no whole footer", path.display()));
    };
    if count != sealed.records || magic != FOOTER_MAGIC {
        return refuse(
            "footer",
            format!("{}'s footer disagrees with its header", path.display()),
        );
    }
    let body_digest = body.finalize();
    if body_digest != stated {
        return refuse(
            "body_digest",
            format!("{}'s body does not match its footer", path.display()),
        );
    }
    Ok(Verified {
        sealed,
        body_digest,
    })
}

// -------------------------------------------------------------- records --

/// A fixed-stride record of one kind and version.
pub trait Record: Sized {
    /// The kind it is stored in.
    const KIND: Kind;
    /// Its layout version.
    const VERSION: u16;
    /// Appends its bytes.
    fn encode(&self, out: &mut Put);
    /// Reads it back.
    ///
    /// # Errors
    ///
    /// A refusal by name for bytes that are not one.
    fn decode(bytes: &[u8]) -> Result<Self, String>;
    /// Its stride, measured by encoding a probe.
    fn stride() -> u32;
}

/// Writes `records` of one [`Record`] type as one sealed file.
///
/// # Errors
///
/// Every [`write_file`] refusal.
pub fn write_records<R: Record>(
    path: &Path,
    run_identity: [u8; DIGEST],
    schema_digest: [u8; DIGEST],
    scope: Scope,
    profile: Vec<u8>,
    records: &[R],
) -> Result<Written, FileError> {
    let header = Header {
        kind: R::KIND,
        record_version: R::VERSION,
        stride: R::stride(),
        schema_digest,
        run_identity,
        scope,
        profile,
    };
    let encoded: Vec<Vec<u8>> = records
        .iter()
        .map(|record| {
            let mut out = Put::default();
            record.encode(&mut out);
            out.0
        })
        .collect();
    write_file(
        path,
        &header,
        encoded.iter().map(|bytes| Ok(bytes.as_slice())),
    )
}

/// Decodes a whole record, refusing bytes left over.
fn whole<R>(
    bytes: &[u8],
    read: impl FnOnce(&mut Take<'_>) -> Result<R, String>,
) -> Result<R, String> {
    let mut take = Take::new(bytes);
    let record = read(&mut take)?;
    if take.left() != 0 {
        return Err(format!("trailing: {} bytes past the record", take.left()));
    }
    Ok(record)
}

/// A probe's encoded length.
fn stride_of<R: Record>(probe: &R) -> u32 {
    let mut out = Put::default();
    probe.encode(&mut out);
    u32::try_from(out.0.len()).unwrap_or(u32::MAX)
}

/// What one minute slot of a day's closes table holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinuteClose {
    /// A real Zerodha minute's close.
    Real(i64),
    /// A padded no-trade minute (FM-F095): STALE for the rank, its price the
    /// previous in-session close.
    Stale(i64),
    /// No minute at all: no value, never zero.
    Absent,
}

/// The previous-close verdict a day carries (FM-F092, RQ-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseVerdict {
    /// Accepted.
    Accepted,
    /// Refused, by its position in the day input's refusal list.
    Refused(u8),
}

/// One record of a `closes` file: a minute slot, or a day's previous close.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseRecord {
    /// The close of minute slot `slot` (minutes after 09:15) on `day`.
    Minute {
        /// The IST epoch day.
        day: i64,
        /// Minutes after 09:15.
        slot: u16,
        /// What the slot holds.
        close: MinuteClose,
    },
    /// The previous close `day` carries.
    Previous {
        /// The IST epoch day it is carried into.
        day: i64,
        /// The close, paisa, before any basis conversion.
        close: i64,
        /// The day it came from.
        from_day: i64,
        /// The last minute-of-day of the session it closed (the regime tag,
        /// FM-F101).
        regime_last_minute: u16,
        /// Whether it was accepted, or why not.
        verdict: CloseVerdict,
    },
    /// `day` has no previous close (a stock's first day), counted.
    NoPrevious {
        /// The IST epoch day.
        day: i64,
    },
}

impl Record for CloseRecord {
    const KIND: Kind = Kind::Closes;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        let (tag, day, slot, close, from_day, regime, verdict) = match *self {
            Self::Minute { day, slot, close } => {
                let (tag, price) = match close {
                    MinuteClose::Real(price) => (0, price),
                    MinuteClose::Stale(price) => (1, price),
                    MinuteClose::Absent => (CLOSE_ABSENT, 0),
                };
                (tag, day, slot, price, 0, 0, 0)
            }
            Self::Previous {
                day,
                close,
                from_day,
                regime_last_minute,
                verdict,
            } => {
                let verdict = match verdict {
                    CloseVerdict::Accepted => 0,
                    CloseVerdict::Refused(at) => at.saturating_add(1),
                };
                (
                    CLOSE_PREVIOUS,
                    day,
                    0,
                    close,
                    from_day,
                    regime_last_minute,
                    verdict,
                )
            }
            Self::NoPrevious { day } => (CLOSE_NO_PREVIOUS, day, 0, 0, 0, 0, 0),
        };
        out.u8(tag);
        out.i64(day);
        out.u16(slot);
        out.i64(close);
        out.i64(from_day);
        out.u16(regime);
        out.u8(verdict);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| {
            let (tag, day, slot, close, from_day, regime, verdict) = (
                take.u8()?,
                take.i64()?,
                take.u16()?,
                take.i64()?,
                take.i64()?,
                take.u16()?,
                take.u8()?,
            );
            let zero_tail = from_day == 0 && regime == 0 && verdict == 0;
            match tag {
                0 if zero_tail => Ok(Self::Minute {
                    day,
                    slot,
                    close: MinuteClose::Real(close),
                }),
                1 if zero_tail => Ok(Self::Minute {
                    day,
                    slot,
                    close: MinuteClose::Stale(close),
                }),
                _ if tag == CLOSE_ABSENT && zero_tail && close == 0 => Ok(Self::Minute {
                    day,
                    slot,
                    close: MinuteClose::Absent,
                }),
                _ if tag == CLOSE_PREVIOUS && slot == 0 => Ok(Self::Previous {
                    day,
                    close,
                    from_day,
                    regime_last_minute: regime,
                    verdict: verdict
                        .checked_sub(1)
                        .map_or(CloseVerdict::Accepted, CloseVerdict::Refused),
                }),
                _ if tag == CLOSE_NO_PREVIOUS && slot == 0 && close == 0 && zero_tail => {
                    Ok(Self::NoPrevious { day })
                }
                other => Err(format!("not a closes record: tag {other}")),
            }
        })
    }

    fn stride() -> u32 {
        stride_of(&Self::NoPrevious { day: 0 })
    }
}

/// The closes record tags past the two price tags.
const CLOSE_ABSENT: u8 = 1 + 1;
const CLOSE_PREVIOUS: u8 = CLOSE_ABSENT + 1;
const CLOSE_NO_PREVIOUS: u8 = CLOSE_PREVIOUS + 1;

/// One cell of the reference table: one index at one minute of one day
/// (FM-F087). Record `((day - 1) × minutes + minute) × indices + slot`, the
/// index list in the header's profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceCell {
    /// The index's exact minute close and the previous tradable day's last
    /// minute close it is measured against.
    Exact {
        /// The minute's close, paisa.
        close: i64,
        /// The previous tradable day's last minute close, paisa.
        previous_close: i64,
    },
    /// The index's minute close; no previous close to measure a move from.
    NoPrevious {
        /// The minute's close, paisa.
        close: i64,
    },
    /// No stamp, and why, by its position in the reference module's reasons.
    Absent(u8),
}

impl Record for ReferenceCell {
    const KIND: Kind = Kind::Reference;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        let (tag, close, previous) = match *self {
            Self::Exact {
                close,
                previous_close,
            } => (0, close, previous_close),
            Self::NoPrevious { close } => (1, close, 0),
            Self::Absent(reason) => (reason.saturating_add(1 + 1), 0, 0),
        };
        out.u8(tag);
        out.i64(close);
        out.i64(previous);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| {
            let (tag, close, previous) = (take.u8()?, take.i64()?, take.i64()?);
            match (tag, close, previous) {
                (0, close, previous_close) => Ok(Self::Exact {
                    close,
                    previous_close,
                }),
                (1, close, 0) => Ok(Self::NoPrevious { close }),
                (tag, 0, 0) if tag > 1 => Ok(Self::Absent(tag - 1 - 1)),
                _ => Err(format!("not a reference cell: tag {tag}")),
            }
        })
    }

    fn stride() -> u32 {
        stride_of(&Self::Absent(0))
    }
}

/// One stage-A candidate: the FIB-1 `Candidate` and its full cell (v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CandidateRecord {
    /// The candidate.
    pub candidate: indicators::fibflag::types::Candidate,
    /// Its full cell in the FIB cube.
    pub coords: Coords,
}

fn search_byte(search: vocab::fib_cube::Search) -> u8 {
    match search {
        vocab::fib_cube::Search::Long => 0,
        vocab::fib_cube::Search::Short => 1,
    }
}

fn search_of(byte: u8) -> Result<vocab::fib_cube::Search, String> {
    match byte {
        0 => Ok(vocab::fib_cube::Search::Long),
        1 => Ok(vocab::fib_cube::Search::Short),
        other => Err(format!("not a direction: byte {other}")),
    }
}

fn base_kind_of(byte: u8) -> Result<indicators::fibflag::types::BaseKind, String> {
    use indicators::fibflag::types::BaseKind;
    [BaseKind::Micro2, BaseKind::Micro3, BaseKind::Rest]
        .into_iter()
        .find(|kind| kind.cube_value() == byte)
        .ok_or_else(|| format!("not a base kind: byte {byte}"))
}

fn style_of(byte: u8) -> Result<indicators::fibflag::types::EntryStyle, String> {
    use indicators::fibflag::types::EntryStyle;
    [EntryStyle::Close, EntryStyle::Break, EntryStyle::Retest]
        .into_iter()
        .find(|style| style.name_index() == byte)
        .ok_or_else(|| format!("not an entry style: byte {byte}"))
}

impl Record for CandidateRecord {
    const KIND: Kind = Kind::Candidates;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        let c = &self.candidate;
        out.u16(c.stock_slot);
        out.i64(c.ist_day);
        out.u16(c.tf_minutes);
        out.i64(c.decision_close_ts_micros);
        out.u8(c.kind.cube_value());
        out.u8(c.style.name_index());
        out.u8(search_byte(c.direction));
        out.u8(c.base.kind.cube_value());
        out.u8(search_byte(c.base.direction));
        out.i64(c.base.high);
        out.i64(c.base.low);
        out.u16(c.base.candles);
        out.i64(c.base.first_close_ts_micros);
        out.i64(c.base.last_close_ts_micros);
        out.bytes(&self.coords);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        use indicators::fibflag::types::{BaseSnapshot, Candidate};
        whole(bytes, |take| {
            let stock_slot = take.u16()?;
            let ist_day = take.i64()?;
            let tf_minutes = take.u16()?;
            let decision_close_ts_micros = take.i64()?;
            let kind = base_kind_of(take.u8()?)?;
            let style = style_of(take.u8()?)?;
            let direction = search_of(take.u8()?)?;
            let base = BaseSnapshot {
                kind: base_kind_of(take.u8()?)?,
                direction: search_of(take.u8()?)?,
                high: take.i64()?,
                low: take.i64()?,
                candles: take.u16()?,
                first_close_ts_micros: take.i64()?,
                last_close_ts_micros: take.i64()?,
            };
            let mut coords = [0; ROW_COUNT];
            coords.copy_from_slice(take.bytes(ROW_COUNT)?);
            Ok(Self {
                candidate: Candidate {
                    stock_slot,
                    ist_day,
                    tf_minutes,
                    decision_close_ts_micros,
                    kind,
                    style,
                    direction,
                    base,
                },
                coords,
            })
        })
    }

    fn stride() -> u32 {
        use indicators::fibflag::types::{BaseKind, BaseSnapshot, Candidate, EntryStyle};
        let base = BaseSnapshot {
            kind: BaseKind::Rest,
            direction: vocab::fib_cube::Search::Long,
            high: 0,
            low: 0,
            candles: 0,
            first_close_ts_micros: 0,
            last_close_ts_micros: 0,
        };
        stride_of(&Self {
            candidate: Candidate {
                stock_slot: 0,
                ist_day: 0,
                tf_minutes: 0,
                decision_close_ts_micros: 0,
                kind: BaseKind::Rest,
                style: EntryStyle::Close,
                direction: vocab::fib_cube::Search::Long,
                base,
            },
            coords: [0; ROW_COUNT],
        })
    }
}

/// A row's outcome index: the stock-month candidates file and the record
/// ordinal in it (FM-F088), with the row's full cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RowRecord {
    /// The candidates file's series.
    pub series: SeriesId,
    /// Its year.
    pub year: u16,
    /// Its month.
    pub month: u8,
    /// The record ordinal in it.
    pub ordinal: u32,
    /// The row's full cell.
    pub coords: Coords,
}

impl Record for RowRecord {
    const KIND: Kind = Kind::Rows;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        out.u32(self.series.0);
        out.u16(self.year);
        out.u8(self.month);
        out.u32(self.ordinal);
        out.bytes(&self.coords);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| {
            let (series, year, month, ordinal) =
                (SeriesId(take.u32()?), take.u16()?, take.u8()?, take.u32()?);
            let mut coords = [0; ROW_COUNT];
            coords.copy_from_slice(take.bytes(ROW_COUNT)?);
            Ok(Self {
                series,
                year,
                month,
                ordinal,
                coords,
            })
        })
    }

    fn stride() -> u32 {
        stride_of(&Self {
            series: SeriesId(0),
            year: 0,
            month: 0,
            ordinal: 0,
            coords: [0; ROW_COUNT],
        })
    }
}

/// One word of a `marks` bitset: bit `ordinal % 64` of word `ordinal / 64`
/// marks candidate `ordinal` of the stock-month's candidates file (FM-F088).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MarksWord(pub u64);

impl MarksWord {
    /// The word and the bit of a candidate ordinal. O(1).
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    #[must_use]
    pub fn position(ordinal: u64) -> (u64, u64) {
        let bits = u64::from(u64::BITS);
        (ordinal / bits, 1_u64 << (ordinal % bits))
    }

    /// The bitset of `ordinals` over `candidates` ordinals, one word per 64.
    #[must_use]
    pub fn bitset(candidates: u64, ordinals: &[u64]) -> Vec<Self> {
        let words =
            usize::try_from(candidates.div_ceil(u64::from(u64::BITS))).unwrap_or(usize::MAX);
        let mut out = vec![Self(0); words];
        for ordinal in ordinals {
            let (word, bit) = Self::position(*ordinal);
            if let Some(slot) = usize::try_from(word).ok().and_then(|at| out.get_mut(at)) {
                slot.0 |= bit;
            }
        }
        out
    }
}

impl Record for MarksWord {
    const KIND: Kind = Kind::Marks;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        out.u64(self.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| take.u64().map(Self))
    }

    fn stride() -> u32 {
        stride_of(&Self(0))
    }
}

/// Whether candidate `ordinal` is marked in an open `marks` file. O(1): one
/// record read.
///
/// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
///
/// # Errors
///
/// Every [`FibFile::read`] refusal.
pub fn is_marked(marks: &mut FibFile, ordinal: u64) -> Result<bool, FileError> {
    let (word, bit) = MarksWord::position(ordinal);
    Ok(marks.read::<MarksWord>(word)?.0 & bit != 0)
}

/// One stage-DEEP outcome: the candidate it re-walked, the slot and the
/// outcome (v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeepRecord {
    /// The candidate's ordinal in its stock-month candidates file.
    pub ordinal: u32,
    /// The exit rule and trade size.
    pub slot: runner::fib::types::SlotKey,
    /// The outcome.
    pub outcome: runner::fib::types::SlotOutcome,
}

fn exit_reason_byte(reason: runner::fib::types::ExitReason) -> u8 {
    reason as u8
}

fn exit_reason_of(byte: u8) -> Result<runner::fib::types::ExitReason, String> {
    use runner::fib::types::ExitReason;
    [
        ExitReason::StopLoss,
        ExitReason::TrailingStopLoss,
        ExitReason::TrailingTakeProfit,
        ExitReason::TakeProfit,
        ExitReason::Time,
        ExitReason::Forced,
    ]
    .into_iter()
    .find(|reason| exit_reason_byte(*reason) == byte)
    .ok_or_else(|| format!("not an exit reason: byte {byte}"))
}

impl Record for DeepRecord {
    const KIND: Kind = Kind::Deep;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        use runner::fib::types::TradeSize;
        out.u32(self.ordinal);
        out.u32(self.slot.exit.exit_set_version);
        out.u16(self.slot.exit.variant);
        out.u8(match self.slot.size {
            TradeSize::Small => 0,
            TradeSize::Large => 1,
        });
        let o = &self.outcome;
        out.u64(o.filled_qty);
        out.i128(o.entry_value_paisa);
        out.i128(o.exit_value_paisa);
        out.i64(o.risk_per_share_paisa);
        out.u32(o.first_entry_second);
        out.u32(o.last_entry_second);
        out.u32(o.exit_second);
        out.u8(exit_reason_byte(o.exit_reason));
        out.i64(o.mfe_to_exit_paisa);
        out.i64(o.mae_to_exit_paisa);
        out.i64(o.mfe_to_horizon_paisa);
        out.i64(o.mae_to_horizon_paisa);
        out.u32(o.flags.0);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        use runner::fib::types::{ExitRule, SlotKey, SlotOutcome, TradeFlags, TradeSize};
        whole(bytes, |take| {
            let ordinal = take.u32()?;
            let exit = ExitRule {
                exit_set_version: take.u32()?,
                variant: take.u16()?,
            };
            let size = match take.u8()? {
                0 => TradeSize::Small,
                1 => TradeSize::Large,
                other => return Err(format!("not a trade size: byte {other}")),
            };
            let outcome = SlotOutcome {
                filled_qty: take.u64()?,
                entry_value_paisa: take.i128()?,
                exit_value_paisa: take.i128()?,
                risk_per_share_paisa: take.i64()?,
                first_entry_second: take.u32()?,
                last_entry_second: take.u32()?,
                exit_second: take.u32()?,
                exit_reason: exit_reason_of(take.u8()?)?,
                mfe_to_exit_paisa: take.i64()?,
                mae_to_exit_paisa: take.i64()?,
                mfe_to_horizon_paisa: take.i64()?,
                mae_to_horizon_paisa: take.i64()?,
                flags: TradeFlags(take.u32()?),
            };
            Ok(Self {
                ordinal,
                slot: SlotKey { exit, size },
                outcome,
            })
        })
    }

    fn stride() -> u32 {
        use runner::fib::types::{
            ExitReason, ExitRule, SlotKey, SlotOutcome, TradeFlags, TradeSize,
        };
        stride_of(&Self {
            ordinal: 0,
            slot: SlotKey {
                exit: ExitRule {
                    exit_set_version: 0,
                    variant: 0,
                },
                size: TradeSize::Small,
            },
            outcome: SlotOutcome {
                filled_qty: 0,
                entry_value_paisa: 0,
                exit_value_paisa: 0,
                risk_per_share_paisa: 0,
                first_entry_second: 0,
                last_entry_second: 0,
                exit_second: 0,
                exit_reason: ExitReason::Forced,
                mfe_to_exit_paisa: 0,
                mae_to_exit_paisa: 0,
                mfe_to_horizon_paisa: 0,
                mae_to_horizon_paisa: 0,
                flags: TradeFlags::NONE,
            },
        })
    }
}

/// The sealed exam lock (FIB-1 R6): the exam derived once from coverage and
/// written before any learning run. It never moves; its digest enters the
/// identity, and a typed exam that differs is refused by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExamLock {
    /// The exam's first month, as `year × 12 + month - 1`.
    pub from_month: u32,
    /// The exam's last month, the same way.
    pub to_month: u32,
    /// The newest covered day, an IST epoch day (a partial last month is
    /// named by it).
    pub last_covered_day: i64,
    /// The digest of the coverage it was derived from (Zerodha minutes, GDFL
    /// seconds, PIT-1, CAL-1).
    pub coverage_digest: [u8; DIGEST],
}

impl Record for ExamLock {
    const KIND: Kind = Kind::ExamLock;
    const VERSION: u16 = 1;

    fn encode(&self, out: &mut Put) {
        out.u32(self.from_month);
        out.u32(self.to_month);
        out.i64(self.last_covered_day);
        out.bytes(&self.coverage_digest);
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| {
            Ok(Self {
                from_month: take.u32()?,
                to_month: take.u32()?,
                last_covered_day: take.i64()?,
                coverage_digest: take.digest()?,
            })
        })
    }

    fn stride() -> u32 {
        stride_of(&Self {
            from_month: 0,
            to_month: 0,
            last_covered_day: 0,
            coverage_digest: [0; DIGEST],
        })
    }
}

/// The exam-lock file's name under a run's results folder.
pub const EXAM_LOCK_FILE: &str = "exam-lock.fib";

/// Writes the exam lock once under `dir` and returns its file digest, the
/// identity term.
///
/// # Errors
///
/// `already_sealed` when a lock exists (it is written once); every
/// [`write_file`] refusal.
pub fn seal_exam_lock(
    dir: &Path,
    run_identity: [u8; DIGEST],
    schema_digest: [u8; DIGEST],
    lock: ExamLock,
) -> Result<[u8; DIGEST], FileError> {
    write_records(
        &dir.join(EXAM_LOCK_FILE),
        run_identity,
        schema_digest,
        Scope::Run,
        Vec::new(),
        &[lock],
    )
    .map(|written| written.file_digest)
}

/// Reads the sealed exam lock under `dir` and refuses one that differs from
/// `typed` (FIB-1 R6: a typed `--exam-from` that differs is refused by name).
///
/// # Errors
///
/// `exam_lock_differs` naming both; `exam_lock_unreadable` for a lock that
/// is not one whole verified record.
pub fn check_exam_lock(dir: &Path, typed: ExamLock) -> Result<ExamLock, FileError> {
    let path = dir.join(EXAM_LOCK_FILE);
    let sealed = verify_whole(&path)
        .and_then(|_| FibFile::open(&path))
        .and_then(|mut file| {
            if file.records() == 1 {
                file.read::<ExamLock>(0)
            } else {
                refuse("records", format!("{} records, not one", file.records()))
            }
        })
        .map_err(|why| FileError {
            word: "exam_lock_unreadable",
            detail: format!("{}: {why}", path.display()),
        })?;
    if sealed != typed {
        return refuse(
            "exam_lock_differs",
            format!(
                "the sealed exam is {sealed:?}; the run asked for {typed:?}; the exam never moves"
            ),
        );
    }
    Ok(sealed)
}

// --------------------------------------------------------- row segments --

/// One entry of a partition's append-only segment index (FM-F089): the
/// sealed segment of one month, its first global row ordinal, its row count
/// and its file digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SegmentEntry {
    /// The month, as `year × 12 + month - 1`.
    pub month_key: u32,
    /// The partition's global ordinal of the segment's first row.
    pub offset: u64,
    /// Rows in the segment.
    pub count: u64,
    /// blake3 of the segment file.
    pub digest: [u8; DIGEST],
}

impl SegmentEntry {
    fn bytes(&self) -> Vec<u8> {
        let mut out = Put::default();
        out.u32(self.month_key);
        out.u64(self.offset);
        out.u64(self.count);
        out.bytes(&self.digest);
        let own = hash(&out.0);
        out.bytes(&own);
        out.0
    }

    fn of(bytes: &[u8]) -> Result<Self, String> {
        whole(bytes, |take| {
            let entry = Self {
                month_key: take.u32()?,
                offset: take.u64()?,
                count: take.u64()?,
                digest: take.digest()?,
            };
            let stated = take.digest()?;
            if entry.bytes().get(..entry.bytes().len() - DIGEST).map(hash) == Some(stated) {
                Ok(entry)
            } else {
                Err("segment_entry_digest: an index entry does not match its own digest".to_owned())
            }
        })
    }

    /// An entry's length in the index.
    #[must_use]
    pub fn stride() -> usize {
        Self {
            month_key: 0,
            offset: 0,
            count: 0,
            digest: [0; DIGEST],
        }
        .bytes()
        .len()
    }
}

/// The month key of a year and month: `year × 12 + month - 1`.
#[must_use]
pub fn month_key(year: u16, month: u8) -> u32 {
    let months = u32::try_from(vocab::fib_cube::MONTHS_PER_GATE_YEAR).unwrap_or(u32::MAX);
    u32::from(year) * months + u32::from(month).saturating_sub(1)
}

/// A partition's sealed segment of one month.
#[must_use]
pub fn segment_path(dir: &Path, partition: u32, year: u16, month: u8) -> PathBuf {
    dir.join(format!("rows-p{partition}-{year:04}-{month:02}.fib"))
}

/// A partition's append-only segment index.
#[must_use]
pub fn segment_index_path(dir: &Path, partition: u32) -> PathBuf {
    dir.join(format!("rows-p{partition}.segments"))
}

/// Seals a partition's month of rows as one new segment and appends its
/// entry to the partition's index: old segments and old entries are never
/// touched, so a new month costs one segment and one entry.
///
/// # Errors
///
/// `month_not_after` when the month is not after the index's last; every
/// [`read_segment_index`] and [`write_records`] refusal; `io` for the
/// append.
pub fn append_segment(
    dir: &Path,
    partition: u32,
    (year, month): (u16, u8),
    run_identity: [u8; DIGEST],
    schema_digest: [u8; DIGEST],
    rows: &[RowRecord],
) -> Result<SegmentEntry, FileError> {
    let index_path = segment_index_path(dir, partition);
    let entries = match fs::symlink_metadata(&index_path) {
        Ok(_) => read_segment_index(&index_path)?,
        Err(_) => Vec::new(),
    };
    let key = month_key(year, month);
    let offset = match entries.last() {
        Some(last) if last.month_key >= key => {
            return refuse(
                "month_not_after",
                format!(
                    "partition {partition} already holds month key {}",
                    last.month_key
                ),
            );
        }
        Some(last) => last.offset + last.count,
        None => 0,
    };
    let written = write_records(
        &segment_path(dir, partition, year, month),
        run_identity,
        schema_digest,
        Scope::PartitionMonth {
            partition,
            year,
            month,
        },
        Vec::new(),
        rows,
    )?;
    let entry = SegmentEntry {
        month_key: key,
        offset,
        count: written.records,
        digest: written.file_digest,
    };
    let mut index = File::options()
        .create(true)
        .append(true)
        .open(&index_path)
        .map_err(|why| io(&index_path, "open the segment index", &why))?;
    index
        .write_all(&entry.bytes())
        .and_then(|()| index.sync_data())
        .map_err(|why| io(&index_path, "append to the segment index", &why))?;
    sync_directory(&index_path)?;
    Ok(entry)
}

/// Every entry of a partition's segment index, each checked against its own
/// digest.
///
/// # Errors
///
/// `segment_index_torn` for a tail that is not a whole entry;
/// `segment_entry_digest`; `io`.
pub fn read_segment_index(path: &Path) -> Result<Vec<SegmentEntry>, FileError> {
    let bytes = fs::read(path).map_err(|why| io(path, "read the segment index", &why))?;
    let stride = SegmentEntry::stride();
    if bytes.len() % stride != 0 {
        return refuse(
            "segment_index_torn",
            format!("{} ends inside an entry", path.display()),
        );
    }
    bytes
        .chunks(stride)
        .map(|chunk| {
            SegmentEntry::of(chunk).map_err(|why| FileError {
                word: "segment_entry_digest",
                detail: format!("{}: {why}", path.display()),
            })
        })
        .collect()
}

/// Row `ordinal` of a partition's month. O(1): the month names its sealed
/// segment by path, and the ordinal is one record read in it.
///
/// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
///
/// # Errors
///
/// Every [`FibFile::open`] and [`FibFile::read`] refusal.
pub fn seek_row(
    dir: &Path,
    partition: u32,
    (year, month): (u16, u8),
    ordinal: u64,
) -> Result<RowRecord, FileError> {
    FibFile::open(&segment_path(dir, partition, year, month))?.read::<RowRecord>(ordinal)
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
