//! The sealed month: `.sbar` version 1, one whole file written once.
//!
//! A closed month of a series may be held in a second **form**: every
//! committed record of the raw month (`.bin`, `docs/02-store-format.md` §1 to
//! §7), its committed header and its provenance image, packed into one file
//! that is never opened for writing again (D-4965). The form is not the
//! content: the generation pointer (D-3350) still says which content is served,
//! and inside that generation's directory the form says how it is stored.
//!
//! This module is the file and nothing else: [`Builder`] turns a raw month's
//! committed header, records, `.prv` image and (A4) a month summary into the
//! bytes of a `.sbar`, and [`SealedMonth`] reads one back by positional reads.
//! It writes no file, takes no lock and moves nothing: writing the bytes by
//! rename is the sealer's (SC-6), and opening the sealed form instead of the
//! raw one is the read door's (SC-3).
//!
//! # The file
//!
//! ```text
//! 0        512              table_off         prv_off          sum_off        blocks_off        file_len
//! ├─ HEAD ──┼─ DIRECTORY ─────┼─ TIME TABLE ─────┼─ PROVENANCE ───┼─ SUMMARY ──────┼─ FRAMES ─────────┤
//!            24 × n_blocks     8 × (entries+1)    packed .prv      R3-06's bytes    codec 1, back to back
//! ```
//!
//! Every integer is little-endian; every checksum is CRC-32C. The file is a
//! **pure function** of the month's committed records, its committed header,
//! its path, its `.prv` image and the summary handed in: no time, no ordinal,
//! no build stamp. The same month built twice, on any host, is the same bytes.
//!
//! * **HEAD**, 512 bytes ([`Head`]). Bytes 0 to 63 have a bar header slot's
//!   shape — magic, version, stride, flags, generation, `n_valid`, first and
//!   last stamp, symbol id, timeframe and the slot's own CRC at 56 — so the
//!   version is found before the layout is known, and [`Head::bar_header`]
//!   hands `generation::Pointer::describes` the raw month's header unchanged.
//!   The rest names the geometry, the BLAKE3 of the records exactly as the
//!   raw month held them, the CRC-32C of the raw month's regenerated `.crc`
//!   image, the path digest the `.prv` header carries, six column ranges and
//!   the six extremes a `.srt` run header holds. A CRC-32C over bytes 0 to 507
//!   ends it. Every field a reader can derive (offsets, block count, table
//!   size, origin) is derived again and refused on any difference.
//! * **DIRECTORY**, 24 bytes a block: where the frame ends (from
//!   `blocks_off`), the frame's CRC-32C, the stamp of its first row, four zero
//!   bytes, and a CRC-32C over those 20 bytes followed by the block's index
//!   as 8 bytes. **Row to block is a division**, `k = row / block_rows`, and
//!   the entries `k - 1` and `k` are one read of 48 bytes. The keyed checksum
//!   binds an entry to its place, so a reader verifies exactly the entries it
//!   reads and never the whole directory.
//! * **TIME TABLE**, present when the month has more than 64 blocks: entry
//!   `s` is the block holding the first row stamped at or after
//!   `origin + s · 4096 · timeframe`, or `n_blocks`; each is a `u32` and a
//!   CRC-32C over it followed by `s` as 8 bytes. `first_at_or_after` reads two
//!   table entries, the directory entries between them, and one frame's stamps.
//! * **PROVENANCE**: the `.prv` image (D-3334) byte for byte, packed — each
//!   byte combined by XOR with the byte 256 before it, then zero runs as `(0x00, run-1)`
//!   for runs up to 256 and any other byte literal — then the CRC-32C of the
//!   packed bytes.
//! * **SUMMARY** (amendment A4 of round 3): a month summary's version (`u16`,
//!   at least 1), two zero bytes, its bytes, and a CRC-32C over everything
//!   before it. This module carries the bytes and never reads them: their
//!   meaning is R3-06's, and a new summary version is a new number in the same
//!   section, not a new `.sbar` version.
//! * **FRAMES**: codec 1 frames of `block_rows` rows (D-4939,
//!   [`crate::sealed_codec`]), the last one shorter.
//!
//! # What a reader refuses, by name
//!
//! Every refusal is a [`SealedError`] with a stable [`SealedError::reason`],
//! never a panic: a short file, another magic or version, a head or slot that
//! fails its CRC, a flag or reserved byte this version does not define, a
//! geometry the head's own counts do not give, a file of another length than
//! the head states, a head written for another path, symbol, rung or month, a
//! directory or table entry that fails its keyed CRC or runs backwards, a
//! frame that fails its CRC or decodes to another row count or first stamp,
//! and a provenance or summary section that fails its CRC or its length.
//! [`SealedMonth::audit`] decodes the whole month and refuses any difference
//! from the head's digests, ranges, extremes and table. Each refusal is one
//! `store.sealed` error on the log.
//!
//! # Cost
//!
//! Opening reads the head (512 bytes). A row, a block or a page of up to
//! [`MAX_RANGE_ROWS`] rows is one read of the adjacent directory entries and
//! one read of the adjacent frames; a time lookup is one table read, one
//! directory read and one frame's stamps. None of them grows with the month
//! (`docs/06-limits.md` L-SC-1 is the frame; L-SC-16 to L-SC-18 are this
//! file's). Building and auditing are O(the month), once.
//!
//! Specification: `state/design/store-compression-20261012.md` §4.2, §4.4,
//! §6 and §9 (SC-2); decision D-4965.

use crate::crc::{crc32c, crc32c_split};
use crate::format::{BLOCK_LEN, Bar, FLAG_CHECKSUMS, FORMAT_VERSION, OI_NULL, RECORD_LEN};
use crate::header::Header;
use crate::path::{StorePath, YearMonth};
use crate::provenance;
use crate::sealed_codec::{self, BLOCK_ROWS, FrameError};
use crate::sort_index::{Extreme, RunExtremes};
use brutex_core::blake3;
use std::fmt;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;

/// The magic a sealed month starts with. Shares no prefix with a bar file's
/// `BRUTEXB`, so a build that predates this format refuses a `.sbar` opened as
/// bars by its magic rather than guessing.
pub const MAGIC: [u8; 8] = *b"BRUTEXC1";

/// The only `.sbar` version this build writes and reads.
pub const VERSION: u16 = 1;

/// Bytes of the head.
pub const HEAD_LEN: u64 = 512;

/// What a frame decodes to: one 56-byte bar record.
pub const STRIDE: u16 = 56;

/// Bytes of one directory entry.
pub const ENTRY_LEN: u64 = 24;

/// Bytes of one time-table entry.
pub const TABLE_ENTRY_LEN: u64 = 8;

/// Rung slots one time-table entry spans.
pub const TABLE_SLOTS: u32 = 4096;

/// A month of more blocks than this carries a time table; at or below it a
/// lookup bisects the whole directory in one read of at most 1,536 bytes.
pub const TABLE_AFTER_BLOCKS: u64 = 64;

/// The raw bar version a sealed month is made from. Version 2 holds no block
/// checksums, so nothing proves its bytes and it is not sealed.
pub const SEALED_FROM_VERSION: u16 = FORMAT_VERSION;

/// Head flag: a time table is present.
pub const FLAG_TIME_TABLE: u32 = 0b001;

/// Head flag: a provenance section is present.
pub const FLAG_PROVENANCE: u32 = 0b010;

/// Head flag: a month-summary section is present (A4, R3-06's bytes).
pub const FLAG_SUMMARY: u32 = 0b100;

/// Every head flag version 1 knows. Any other bit is refused.
pub const KNOWN_FLAGS: u32 = 0b111;

/// Bytes of the path digest: the first 24 of BLAKE3 over the month's `.bin`
/// store path, the digest a `.prv` header carries.
pub const PATH_DIGEST_LEN: usize = 24;

/// The longest provenance image a sealed month carries: a full `.prv`.
pub const MAX_PROVENANCE_LEN: u64 = provenance::MAX_FILE_LEN;

/// The longest month summary a sealed month carries, its bytes alone.
pub const MAX_SUMMARY_LEN: usize = 1 << 20;

/// Bytes a summary section adds to its summary: version, zero, checksum.
pub const SUMMARY_OVERHEAD: u64 = 8;

/// The most rows one [`SealedMonth::read_rows`] serves: 64 frames of 256.
/// A page of the API is at most 1,000.
pub const MAX_RANGE_ROWS: u64 = 16_384;

/// A time lookup bisects directory entries read in one positional read when
/// it has at most this many to consider; past it (a month holding two bars in
/// one slot, written before D-0915) it reads one entry per bisection step.
pub const BISECT_IN_ONE_READ: u64 = 64;

/// The six column ranges, in the order the head stores them.
pub const COLUMN_NAMES: [&str; 6] = ["open", "high", "low", "close", "volume", "open_interest"];

/// The six extremes, in the order the head stores them (a `.srt` run
/// header's order, D-3326).
pub const EXTREME_NAMES: [&str; 6] = [
    "close_max",
    "close_min",
    "high_max",
    "low_min",
    "range_max",
    "volume_max",
];

const HEAD: usize = 512;
const OFF_SLOT_CRC: usize = 56;
const OFF_SLOT_ZERO: usize = 60;
const OFF_COLUMNS: usize = 224;
const OFF_EXTREMES: usize = 368;
const OFF_RESERVED: usize = 464;
const OFF_HEAD_CRC: usize = 508;
const ENTRY: usize = 24;
const TABLE_ENTRY: usize = 8;
const MICROS: i128 = 1_000_000;

/// One column's range over the month: its nulls, and its smallest and largest
/// non-null value — both zero when every row is null or the month is empty.
/// Only open interest has a null (`i64::MIN`); the other five count none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnRange {
    /// Rows holding the column's null.
    pub nulls: u64,
    /// The smallest non-null value.
    pub smallest: i64,
    /// The largest non-null value.
    pub largest: i64,
}

/// The 512-byte head of a sealed month, field by field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    /// [`FLAG_TIME_TABLE`], [`FLAG_PROVENANCE`], [`FLAG_SUMMARY`].
    pub flags: u32,
    /// The raw month's committed header generation, copied.
    pub generation: u64,
    /// Committed records.
    pub n_valid: u64,
    /// Stamp of row 0, copied from the raw header.
    pub first_ts_micros: i64,
    /// Stamp of row `n_valid - 1`, copied from the raw header.
    pub last_ts_micros: i64,
    /// The bar header's symbol id.
    pub symbol_id: u32,
    /// Seconds per bar.
    pub timeframe_secs: u32,
    /// Rows a full frame holds; read, never assumed.
    pub block_rows: u16,
    /// The raw bar version the month was sealed from.
    pub raw_version: u16,
    /// Frames: `ceil(n_valid / block_rows)`.
    pub n_blocks: u32,
    /// Time-table entries less one; zero when absent.
    pub table_entries: u32,
    /// Bytes of the packed provenance, its checksum not counted.
    pub prv_packed_len: u32,
    /// Bytes of the provenance image before packing.
    pub prv_raw_len: u32,
    /// The month's year.
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// Where the time table starts.
    pub table_off: u64,
    /// Where the provenance section starts.
    pub prv_off: u64,
    /// Where the frames start.
    pub blocks_off: u64,
    /// Bytes of all frames.
    pub blocks_len: u64,
    /// Bytes of the whole file.
    pub file_len: u64,
    /// 00:00 IST on the 1st of the month, epoch microseconds.
    pub origin_micros: i64,
    /// Where the summary section starts; zero when absent.
    pub sum_off: u64,
    /// Bytes of the summary section; zero when absent.
    pub sum_len: u32,
    /// BLAKE3 of the `n_valid × 56` record bytes exactly as the raw month
    /// held them.
    pub records_blake3: [u8; 32],
    /// CRC-32C of the raw month's `.crc` image over blocks
    /// `0..blocks_for(n_valid)`, regenerated from the records.
    pub crc_image_crc: u32,
    /// The first 24 bytes of BLAKE3 over the month's `.bin` store path.
    pub path_digest: [u8; PATH_DIGEST_LEN],
    /// Open, high, low, close, volume, open interest.
    pub columns: [ColumnRange; 6],
    /// Each with the stamp of the EARLIEST bar holding it.
    pub extremes: RunExtremes,
}

/// A month summary handed to the builder: R3-06's bytes and their version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary<'a> {
    /// The summary's own format version, at least 1.
    pub version: u16,
    /// Its bytes, at most [`MAX_SUMMARY_LEN`].
    pub bytes: &'a [u8],
}

/// Why a sealed month was refused, built or read. Each has its own name.
#[derive(Debug)]
pub enum SealedError {
    /// The file is shorter than its head.
    TooShort {
        /// Bytes the file holds.
        len: u64,
    },
    /// The first eight bytes are not [`MAGIC`].
    Magic,
    /// A version this build does not read.
    Version {
        /// The version found.
        found: u16,
    },
    /// The head's CRC over bytes 0 to 507 does not match.
    HeadChecksum {
        /// What the head stores.
        stored: u32,
        /// What its bytes hash to.
        computed: u32,
    },
    /// The slot-shaped first 64 bytes fail their own CRC.
    SlotChecksum {
        /// What the slot stores.
        stored: u32,
        /// What its bytes hash to.
        computed: u32,
    },
    /// A record stride other than 56.
    Stride {
        /// The stride found.
        found: u16,
    },
    /// A head flag this version does not define.
    Flags {
        /// The flags found.
        found: u32,
    },
    /// A reserved byte is set.
    Reserved {
        /// The first set byte's offset in the head.
        offset: usize,
    },
    /// Rows a frame holds is 0 or above [`BLOCK_ROWS`].
    BlockRows {
        /// The count found.
        found: u16,
    },
    /// A codec other than 1.
    Codec {
        /// The codec found.
        found: u16,
    },
    /// A directory entry length other than 24.
    EntryLen {
        /// The length found.
        found: u16,
    },
    /// A raw bar version other than 3.
    RawVersion {
        /// The version found.
        found: u16,
    },
    /// A raw month of version 2: no block checksums prove its bytes.
    UnverifiedSource,
    /// A raw header whose flags are not exactly "block checksums present".
    RawFlags {
        /// The flags found.
        found: u32,
    },
    /// A time table of other than 4,096 slots an entry.
    TableSlots {
        /// The count found.
        found: u32,
    },
    /// A head field disagrees with what the head's own counts give.
    Geometry {
        /// The field.
        field: &'static str,
        /// What the head states.
        head: u64,
        /// What its counts give.
        expected: u64,
    },
    /// The file is not the length its head states.
    Length {
        /// What the head states.
        head: u64,
        /// What the file holds.
        file: u64,
    },
    /// The head names another path, symbol, rung or month.
    Misfiled {
        /// The field that differs.
        field: &'static str,
    },
    /// The head's first and last stamps run backwards or leave the month.
    Stamps {
        /// The first stamp stated.
        first: i64,
        /// The last stamp stated.
        last: i64,
    },
    /// A column range is not one a month can hold.
    Columns {
        /// The column.
        column: &'static str,
    },
    /// An extreme disagrees with the column ranges or the month's stamps.
    Extremes {
        /// The extreme.
        extreme: &'static str,
    },
    /// A directory entry fails its keyed CRC or sets a zero byte.
    Entry {
        /// The entry's index.
        index: u64,
    },
    /// A frame's span runs backwards, past the frames or past its bound.
    EntryOrder {
        /// The block.
        block: u64,
    },
    /// A frame was refused by codec 1.
    Frame {
        /// The block.
        block: u64,
        /// The codec's refusal.
        why: FrameError,
    },
    /// A frame decoded to another row count than its block holds.
    FrameRows {
        /// The block.
        block: u64,
        /// Rows the block holds.
        expected: u64,
        /// Rows the frame holds.
        found: u64,
    },
    /// A frame's first stamp is not the one its directory entry states.
    FirstStamp {
        /// The block.
        block: u64,
    },
    /// A time-table entry fails its keyed CRC.
    Table {
        /// The entry's index.
        index: u64,
    },
    /// A time-table entry names a block past the month or before the entry
    /// it follows.
    TableOrder {
        /// The entry's index.
        index: u64,
    },
    /// The provenance section fails its checksum or its length.
    Provenance {
        /// Which: `checksum`, `truncated`, `overrun` or `short`.
        why: &'static str,
    },
    /// The summary section fails its checksum or its shape.
    Summary {
        /// Which: `checksum`, `version` or `reserved`.
        why: &'static str,
    },
    /// Rows asked past the month, or more than [`MAX_RANGE_ROWS`] at once.
    Range {
        /// The first row asked.
        first: u64,
        /// Rows asked.
        count: u64,
        /// Rows the month holds.
        n_valid: u64,
    },
    /// The host refused a read.
    Io {
        /// What was being read.
        action: &'static str,
        /// Where.
        offset: u64,
        /// The host's refusal.
        source: io::Error,
    },
    /// A whole-month decode disagrees with the head.
    Audit {
        /// The head field that differs.
        field: &'static str,
    },
    /// A row's stamp is before the row before it.
    OutOfOrder {
        /// The row.
        row: u64,
    },
    /// A row's stamp is outside the month's IST span.
    OutsideMonth {
        /// The row.
        row: u64,
    },
    /// More rows offered than the raw header commits.
    TooManyRows {
        /// Rows the raw header commits.
        n_valid: u64,
    },
    /// Fewer rows offered than the raw header commits.
    Count {
        /// Rows the raw header commits.
        expected: u64,
        /// Rows offered.
        found: u64,
    },
    /// The rows offered do not start and end at the raw header's stamps.
    HeaderStamps,
    /// The month needs more blocks or frame bytes than the format addresses.
    TooLarge,
    /// A provenance image longer than a `.prv` can be.
    ProvenanceTooLong {
        /// Its length.
        len: u64,
    },
    /// A provenance image whose header names another month, rung or path.
    ProvenanceForeign,
    /// A month summary longer than [`MAX_SUMMARY_LEN`].
    SummaryTooLong {
        /// Its length.
        len: u64,
    },
    /// A month summary of version 0.
    SummaryVersion,
}

impl SealedError {
    /// A stable name for the refusal, for an event, a journal or a test.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::TooShort { .. } => "too_short",
            Self::Magic => "magic",
            Self::Version { .. } => "version",
            Self::HeadChecksum { .. } => "head_checksum",
            Self::SlotChecksum { .. } => "slot_checksum",
            Self::Stride { .. } => "stride",
            Self::Flags { .. } => "flags",
            Self::Reserved { .. } => "reserved",
            Self::BlockRows { .. } => "block_rows",
            Self::Codec { .. } => "codec",
            Self::EntryLen { .. } => "entry_len",
            Self::RawVersion { .. } => "raw_version",
            Self::UnverifiedSource => "unverified_source",
            Self::RawFlags { .. } => "raw_flags",
            Self::TableSlots { .. } => "table_slots",
            Self::Geometry { .. } => "geometry",
            Self::Length { .. } => "length",
            Self::Misfiled { .. } => "misfiled",
            Self::Stamps { .. } => "stamps",
            Self::Columns { .. } => "columns",
            Self::Extremes { .. } => "extremes",
            Self::Entry { .. } => "entry_checksum",
            Self::EntryOrder { .. } => "entry_order",
            Self::Frame { .. } => "frame",
            Self::FrameRows { .. } => "frame_rows",
            Self::FirstStamp { .. } => "first_stamp",
            Self::Table { .. } => "table_checksum",
            Self::TableOrder { .. } => "table_order",
            Self::Provenance { .. } => "provenance",
            Self::Summary { .. } => "summary",
            Self::Range { .. } => "range",
            Self::Io { .. } => "io",
            Self::Audit { .. } => "audit",
            Self::OutOfOrder { .. } => "out_of_order",
            Self::OutsideMonth { .. } => "outside_month",
            Self::TooManyRows { .. } => "too_many_rows",
            Self::Count { .. } => "count",
            Self::HeaderStamps => "header_stamps",
            Self::TooLarge => "too_large",
            Self::ProvenanceTooLong { .. } => "provenance_too_long",
            Self::ProvenanceForeign => "provenance_foreign",
            Self::SummaryTooLong { .. } => "summary_too_long",
            Self::SummaryVersion => "summary_version",
        }
    }
}

impl fmt::Display for SealedError {
    // One arm a refusal: splitting it would scatter the sentences a reader
    // matches against `reason()`.
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = self.reason();
        match self {
            Self::TooShort { len } => {
                write!(
                    f,
                    "{reason}: {len} bytes, shorter than the {HEAD_LEN}-byte head"
                )
            }
            Self::Magic => write!(f, "{reason}: not a sealed month (no BRUTEXC1)"),
            Self::Version { found } => {
                write!(
                    f,
                    "{reason}: .sbar version {found}; this build reads {VERSION}"
                )
            }
            Self::HeadChecksum { stored, computed } | Self::SlotChecksum { stored, computed } => {
                write!(
                    f,
                    "{reason}: stored {stored:#010x}, bytes hash to {computed:#010x}"
                )
            }
            Self::Stride { found } => write!(f, "{reason}: stride {found}, not {STRIDE}"),
            Self::Flags { found } => write!(
                f,
                "{reason}: flags {found:#x} set bits version 1 does not define"
            ),
            Self::Reserved { offset } => {
                write!(f, "{reason}: reserved byte {offset} of the head is set")
            }
            Self::BlockRows { found } => {
                write!(
                    f,
                    "{reason}: {found} rows a frame; 1 to {BLOCK_ROWS} are read"
                )
            }
            Self::Codec { found } => write!(f, "{reason}: codec {found}, not 1"),
            Self::EntryLen { found } => write!(
                f,
                "{reason}: directory entries of {found} bytes, not {ENTRY_LEN}"
            ),
            Self::RawVersion { found } => {
                write!(
                    f,
                    "{reason}: raw bar version {found}; only {SEALED_FROM_VERSION} is sealed"
                )
            }
            Self::UnverifiedSource => write!(
                f,
                "{reason}: a version 2 month holds no block checksums, so nothing proves the bytes a seal would keep"
            ),
            Self::RawFlags { found } => {
                write!(
                    f,
                    "{reason}: raw header flags {found:#x}, not {FLAG_CHECKSUMS:#x}"
                )
            }
            Self::TableSlots { found } => {
                write!(
                    f,
                    "{reason}: {found} slots a table entry, not {TABLE_SLOTS}"
                )
            }
            Self::Geometry {
                field,
                head,
                expected,
            } => write!(
                f,
                "{reason}: {field} is {head} in the head; its counts give {expected}"
            ),
            Self::Length { head, file } => {
                write!(
                    f,
                    "{reason}: the head states {head} bytes, the file holds {file}"
                )
            }
            Self::Misfiled { field } => {
                write!(f, "{reason}: the head's {field} is not this month's")
            }
            Self::Stamps { first, last } => write!(
                f,
                "{reason}: first stamp {first} and last stamp {last} run backwards or leave the month"
            ),
            Self::Columns { column } => {
                write!(
                    f,
                    "{reason}: the {column} range is not one a month can hold"
                )
            }
            Self::Extremes { extreme } => {
                write!(
                    f,
                    "{reason}: {extreme} disagrees with the column ranges or the stamps"
                )
            }
            Self::Entry { index } => {
                write!(
                    f,
                    "{reason}: directory entry {index} fails its keyed checksum"
                )
            }
            Self::EntryOrder { block } => write!(
                f,
                "{reason}: frame {block}'s span runs backwards, past the frames or past its bound"
            ),
            Self::Frame { block, why } => write!(f, "{reason}: block {block}: {why}"),
            Self::FrameRows {
                block,
                expected,
                found,
            } => write!(
                f,
                "{reason}: block {block} holds {expected} rows, its frame {found}"
            ),
            Self::FirstStamp { block } => write!(
                f,
                "{reason}: block {block}'s first stamp is not its directory entry's"
            ),
            Self::Table { index } => {
                write!(
                    f,
                    "{reason}: time-table entry {index} fails its keyed checksum"
                )
            }
            Self::TableOrder { index } => write!(
                f,
                "{reason}: time-table entry {index} names a block past the month or before the entry it follows"
            ),
            Self::Provenance { why } => write!(f, "{reason}: the packed provenance: {why}"),
            Self::Summary { why } => write!(f, "{reason}: the month summary section: {why}"),
            Self::Range {
                first,
                count,
                n_valid,
            } => write!(
                f,
                "{reason}: rows {first}+{count} of a month of {n_valid}, at most {MAX_RANGE_ROWS} at once"
            ),
            Self::Io {
                action,
                offset,
                source,
            } => write!(f, "{reason}: {action} at byte {offset}: {source}"),
            Self::Audit { field } => {
                write!(f, "{reason}: the decoded month's {field} is not the head's")
            }
            Self::OutOfOrder { row } => {
                write!(f, "{reason}: row {row}'s stamp is before the row before it")
            }
            Self::OutsideMonth { row } => {
                write!(
                    f,
                    "{reason}: row {row}'s stamp is outside the month's IST span"
                )
            }
            Self::TooManyRows { n_valid } => {
                write!(
                    f,
                    "{reason}: more rows than the {n_valid} the raw header commits"
                )
            }
            Self::Count { expected, found } => {
                write!(
                    f,
                    "{reason}: {found} rows offered, the raw header commits {expected}"
                )
            }
            Self::HeaderStamps => write!(
                f,
                "{reason}: the rows do not start and end at the raw header's stamps"
            ),
            Self::TooLarge => write!(
                f,
                "{reason}: more blocks or frame bytes than a u32 directory addresses"
            ),
            Self::ProvenanceTooLong { len } => {
                write!(
                    f,
                    "{reason}: {len} bytes; a .prv is at most {MAX_PROVENANCE_LEN}"
                )
            }
            Self::ProvenanceForeign => write!(
                f,
                "{reason}: the .prv header names another month, rung, symbol or path"
            ),
            Self::SummaryTooLong { len } => {
                write!(f, "{reason}: {len} bytes; at most {MAX_SUMMARY_LEN}")
            }
            Self::SummaryVersion => write!(f, "{reason}: a month summary's version is at least 1"),
        }
    }
}

impl std::error::Error for SealedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Frame { why, .. } => Some(why),
            _ => None,
        }
    }
}

/// One refusal, on the log. The ONE emit site: every public door of this
/// module reaches it through `inspect_err`, so a later refusal is not the one
/// nobody logged.
fn note_refused(refusal: &SealedError) {
    let why = refusal.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("store.sealed", "sealed month refused")
            .with("reason", telemetry::Value::Str(refusal.reason()))
            .with("why", telemetry::Value::Str(&why)),
    );
}

// ---------------------------------------------------------------------------
// Bytes
// ---------------------------------------------------------------------------

fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(slot) = out.get_mut(at..at + bytes.len()) {
        slot.copy_from_slice(bytes);
    }
}

fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(src) = bytes.get(at..at + N) {
        out.copy_from_slice(src);
    }
    out
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(take(bytes, at))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(take(bytes, at))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(take(bytes, at))
}

fn i64_at(bytes: &[u8], at: usize) -> i64 {
    i64::from_le_bytes(take(bytes, at))
}

fn len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

/// The slot-shaped checksum: CRC-32C over bytes 0 to 55 and 60 to 63, as
/// every bar header slot's.
fn slot_crc(head: &[u8]) -> u32 {
    crc32c_split(
        head.get(..OFF_SLOT_CRC).unwrap_or_default(),
        head.get(OFF_SLOT_ZERO..OFF_SLOT_ZERO + 4)
            .unwrap_or_default(),
    )
}

fn head_crc(head: &[u8]) -> u32 {
    crc32c(head.get(..OFF_HEAD_CRC).unwrap_or_default())
}

/// The bytes of directory entry `index`.
fn entry_image(index: u64, end: u32, frame_crc: u32, first_ts: i64) -> [u8; ENTRY] {
    let mut out = [0u8; ENTRY];
    put(&mut out, 0, &end.to_le_bytes());
    put(&mut out, 4, &frame_crc.to_le_bytes());
    put(&mut out, 8, &first_ts.to_le_bytes());
    let crc = crc32c_split(out.get(..20).unwrap_or_default(), &index.to_le_bytes());
    put(&mut out, 20, &crc.to_le_bytes());
    out
}

/// The bytes of time-table entry `index`.
fn table_image(index: u64, block: u32) -> [u8; TABLE_ENTRY] {
    let mut out = [0u8; TABLE_ENTRY];
    put(&mut out, 0, &block.to_le_bytes());
    let crc = crc32c_split(&block.to_le_bytes(), &index.to_le_bytes());
    put(&mut out, 4, &crc.to_le_bytes());
    out
}

/// One directory entry, checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    end: u32,
    crc: u32,
    first_ts: i64,
}

impl Entry {
    fn decode(bytes: &[u8], index: u64) -> Result<Self, SealedError> {
        let stored = u32_at(bytes, 20);
        let computed = crc32c_split(bytes.get(..20).unwrap_or_default(), &index.to_le_bytes());
        if stored != computed || u32_at(bytes, 16) != 0 {
            return Err(SealedError::Entry { index });
        }
        Ok(Self {
            end: u32_at(bytes, 0),
            crc: u32_at(bytes, 4),
            first_ts: i64_at(bytes, 8),
        })
    }
}

// ---------------------------------------------------------------------------
// Provenance packing
// ---------------------------------------------------------------------------

/// Packs a `.prv` image: each byte combined by XOR with the byte 256 before it (a
/// provenance record is 256 bytes, so equal fields of consecutive records
/// become zeros), then each run of 1 to 256 zeros as `(0x00, run - 1)` and
/// every other byte as itself. Deterministic; at most twice the image.
#[must_use]
pub fn pack_provenance(image: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.len());
    let mut run: u16 = 0;
    for (at, byte) in image.iter().enumerate() {
        let before = at
            .checked_sub(256)
            .and_then(|i| image.get(i))
            .copied()
            .unwrap_or(0);
        let x = byte ^ before;
        if x == 0 {
            run += 1;
            if run == 256 {
                out.extend_from_slice(&[0, 255]);
                run = 0;
            }
        } else {
            flush_run(&mut out, &mut run);
            out.push(x);
        }
    }
    flush_run(&mut out, &mut run);
    out
}

fn flush_run(out: &mut Vec<u8>, run: &mut u16) {
    if let Some(less) = run.checked_sub(1) {
        out.extend_from_slice(&[0, u8::try_from(less).unwrap_or(u8::MAX)]);
    }
    *run = 0;
}

/// Unpacks what [`pack_provenance`] wrote into exactly `raw_len` bytes.
///
/// # Errors
///
/// [`SealedError::Provenance`]: `truncated` for a zero with no run length
/// after it, `overrun` for more bytes than `raw_len`, `short` for fewer.
pub fn unpack_provenance(packed: &[u8], raw_len: usize) -> Result<Vec<u8>, SealedError> {
    let mut out: Vec<u8> = Vec::with_capacity(raw_len);
    let mut bytes = packed.iter();
    while let Some(&byte) = bytes.next() {
        let (value, times) = if byte == 0 {
            let Some(&less) = bytes.next() else {
                return Err(SealedError::Provenance { why: "truncated" });
            };
            (0, usize::from(less) + 1)
        } else {
            (byte, 1)
        };
        if out.len() + times > raw_len {
            return Err(SealedError::Provenance { why: "overrun" });
        }
        for _ in 0..times {
            let before = out
                .len()
                .checked_sub(256)
                .and_then(|i| out.get(i))
                .copied()
                .unwrap_or(0);
            out.push(value ^ before);
        }
    }
    if out.len() == raw_len {
        Ok(out)
    } else {
        Err(SealedError::Provenance { why: "short" })
    }
}

// ---------------------------------------------------------------------------
// The head
// ---------------------------------------------------------------------------

/// Time-table entries (less one) for a month of `span_micros` at `timeframe_secs`.
fn table_entries_for(span_micros: i64, timeframe_secs: u32) -> u64 {
    // Every caller has refused a zero timeframe and an empty span already;
    // the checked division keeps that a value, never a panic.
    let slot = i128::from(TABLE_SLOTS) * i128::from(timeframe_secs) * MICROS;
    (i128::from(span_micros) + slot - 1)
        .checked_div(slot)
        .and_then(|entries| u64::try_from(entries).ok())
        .unwrap_or(0)
}

impl Head {
    /// The raw month's committed header this head copies, as a bar
    /// [`Header`]: what `generation::Pointer::describes` compares a
    /// revision's pointer with, so a sealed revision is described unchanged.
    #[must_use]
    pub const fn bar_header(&self) -> Header {
        Header {
            format_version: self.raw_version,
            record_stride: STRIDE,
            flags: FLAG_CHECKSUMS,
            generation: self.generation,
            n_valid: self.n_valid,
            first_ts_micros: self.first_ts_micros,
            last_ts_micros: self.last_ts_micros,
            symbol_id: self.symbol_id,
            timeframe_secs: self.timeframe_secs,
        }
    }

    /// Rows block `block` holds: `block_rows`, the remainder for the last,
    /// zero past the month.
    #[must_use]
    pub fn rows_in(&self, block: u64) -> u64 {
        let rows = u64::from(self.block_rows);
        let start = block.saturating_mul(rows);
        self.n_valid.saturating_sub(start).min(rows)
    }

    /// The 512 bytes this head is written as, both checksums computed.
    #[must_use]
    pub fn encode(&self) -> [u8; HEAD] {
        let mut b = [0u8; HEAD];
        put(&mut b, 0, &MAGIC);
        put(&mut b, 8, &VERSION.to_le_bytes());
        put(&mut b, 10, &STRIDE.to_le_bytes());
        put(&mut b, 12, &self.flags.to_le_bytes());
        put(&mut b, 16, &self.generation.to_le_bytes());
        put(&mut b, 24, &self.n_valid.to_le_bytes());
        put(&mut b, 32, &self.first_ts_micros.to_le_bytes());
        put(&mut b, 40, &self.last_ts_micros.to_le_bytes());
        put(&mut b, 48, &self.symbol_id.to_le_bytes());
        put(&mut b, 52, &self.timeframe_secs.to_le_bytes());
        put(&mut b, 64, &self.block_rows.to_le_bytes());
        put(&mut b, 66, &u16::from(sealed_codec::CODEC).to_le_bytes());
        put(&mut b, 68, &24u16.to_le_bytes());
        put(&mut b, 70, &self.raw_version.to_le_bytes());
        put(&mut b, 72, &self.n_blocks.to_le_bytes());
        put(&mut b, 76, &TABLE_SLOTS.to_le_bytes());
        put(&mut b, 80, &self.table_entries.to_le_bytes());
        put(&mut b, 84, &self.prv_packed_len.to_le_bytes());
        put(&mut b, 88, &self.prv_raw_len.to_le_bytes());
        put(&mut b, 92, &self.year.to_le_bytes());
        put(&mut b, 94, &[self.month]);
        put(&mut b, 96, &self.table_off.to_le_bytes());
        put(&mut b, 104, &self.prv_off.to_le_bytes());
        put(&mut b, 112, &self.blocks_off.to_le_bytes());
        put(&mut b, 120, &self.blocks_len.to_le_bytes());
        put(&mut b, 128, &self.file_len.to_le_bytes());
        put(&mut b, 136, &self.origin_micros.to_le_bytes());
        put(&mut b, 144, &self.sum_off.to_le_bytes());
        put(&mut b, 152, &self.sum_len.to_le_bytes());
        put(&mut b, 160, &self.records_blake3);
        put(&mut b, 192, &self.crc_image_crc.to_le_bytes());
        put(&mut b, 200, &self.path_digest);
        for (at, column) in self.columns.iter().enumerate() {
            let off = OFF_COLUMNS + 24 * at;
            put(&mut b, off, &column.nulls.to_le_bytes());
            put(&mut b, off + 8, &column.smallest.to_le_bytes());
            put(&mut b, off + 16, &column.largest.to_le_bytes());
        }
        for (at, extreme) in extremes_of(&self.extremes).iter().enumerate() {
            let off = OFF_EXTREMES + 16 * at;
            put(&mut b, off, &extreme.v.to_le_bytes());
            put(&mut b, off + 8, &extreme.ts.to_le_bytes());
        }
        let slot = slot_crc(&b);
        put(&mut b, OFF_SLOT_CRC, &slot.to_le_bytes());
        let whole = head_crc(&b);
        put(&mut b, OFF_HEAD_CRC, &whole.to_le_bytes());
        b
    }

    /// Decodes a head and checks everything it can say about itself: magic,
    /// version, both checksums, stride, flags, reserved bytes, block rows,
    /// codec, entry length, raw version, table slots, every derived offset
    /// and count, the stamps against the month, the column ranges and the
    /// extremes. What it says about its PATH is [`Head::check`]'s.
    ///
    /// # Errors
    ///
    /// The [`SealedError`] naming the first field that fails, checked in the
    /// order a reader can trust each: the length, the magic and the version
    /// first, then the head's checksum, so a rotted head is called that and
    /// not a strange geometry.
    pub fn decode(bytes: &[u8]) -> Result<Self, SealedError> {
        if bytes.len() < HEAD {
            return Err(SealedError::TooShort {
                len: len_u64(bytes.len()),
            });
        }
        if take::<8>(bytes, 0) != MAGIC {
            return Err(SealedError::Magic);
        }
        let version = u16_at(bytes, 8);
        if version != VERSION {
            return Err(SealedError::Version { found: version });
        }
        let stored = u32_at(bytes, OFF_HEAD_CRC);
        let computed = head_crc(bytes);
        if stored != computed {
            return Err(SealedError::HeadChecksum { stored, computed });
        }
        let stored = u32_at(bytes, OFF_SLOT_CRC);
        let computed = slot_crc(bytes);
        if stored != computed {
            return Err(SealedError::SlotChecksum { stored, computed });
        }
        Self::check_constants(bytes)?;
        let head = Self::fields(bytes);
        head.check_geometry()?;
        head.check_contents()?;
        Ok(head)
    }

    /// The fields a version-1 head fixes, and the reserved bytes.
    fn check_constants(bytes: &[u8]) -> Result<(), SealedError> {
        let stride = u16_at(bytes, 10);
        if stride != STRIDE {
            return Err(SealedError::Stride { found: stride });
        }
        let flags = u32_at(bytes, 12);
        if flags & !KNOWN_FLAGS != 0 {
            return Err(SealedError::Flags { found: flags });
        }
        let reserved = [
            OFF_SLOT_ZERO..OFF_SLOT_ZERO + 4,
            95..96,
            156..160,
            196..200,
            OFF_RESERVED..OFF_HEAD_CRC,
        ];
        for mut range in reserved {
            if let Some(offset) = range.find(|&at| bytes.get(at) != Some(&0)) {
                return Err(SealedError::Reserved { offset });
            }
        }
        let block_rows = u16_at(bytes, 64);
        if block_rows == 0 || block_rows > BLOCK_ROWS {
            return Err(SealedError::BlockRows { found: block_rows });
        }
        let codec = u16_at(bytes, 66);
        if codec != u16::from(sealed_codec::CODEC) {
            return Err(SealedError::Codec { found: codec });
        }
        let entry = u16_at(bytes, 68);
        if u64::from(entry) != ENTRY_LEN {
            return Err(SealedError::EntryLen { found: entry });
        }
        let raw = u16_at(bytes, 70);
        if raw != SEALED_FROM_VERSION {
            return Err(SealedError::RawVersion { found: raw });
        }
        let slots = u32_at(bytes, 76);
        if slots != TABLE_SLOTS {
            return Err(SealedError::TableSlots { found: slots });
        }
        Ok(())
    }

    fn fields(bytes: &[u8]) -> Self {
        let mut columns = [ColumnRange::default(); 6];
        for (at, column) in columns.iter_mut().enumerate() {
            let off = OFF_COLUMNS + 24 * at;
            *column = ColumnRange {
                nulls: u64_at(bytes, off),
                smallest: i64_at(bytes, off + 8),
                largest: i64_at(bytes, off + 16),
            };
        }
        let extreme = |at: usize| Extreme {
            v: i64_at(bytes, OFF_EXTREMES + 16 * at),
            ts: i64_at(bytes, OFF_EXTREMES + 16 * at + 8),
        };
        let mut records_blake3 = [0u8; 32];
        put(
            &mut records_blake3,
            0,
            bytes.get(160..192).unwrap_or_default(),
        );
        let mut path_digest = [0u8; PATH_DIGEST_LEN];
        put(&mut path_digest, 0, bytes.get(200..224).unwrap_or_default());
        Self {
            flags: u32_at(bytes, 12),
            generation: u64_at(bytes, 16),
            n_valid: u64_at(bytes, 24),
            first_ts_micros: i64_at(bytes, 32),
            last_ts_micros: i64_at(bytes, 40),
            symbol_id: u32_at(bytes, 48),
            timeframe_secs: u32_at(bytes, 52),
            block_rows: u16_at(bytes, 64),
            raw_version: u16_at(bytes, 70),
            n_blocks: u32_at(bytes, 72),
            table_entries: u32_at(bytes, 80),
            prv_packed_len: u32_at(bytes, 84),
            prv_raw_len: u32_at(bytes, 88),
            year: u16_at(bytes, 92),
            month: bytes.get(94).copied().unwrap_or(0),
            table_off: u64_at(bytes, 96),
            prv_off: u64_at(bytes, 104),
            blocks_off: u64_at(bytes, 112),
            blocks_len: u64_at(bytes, 120),
            file_len: u64_at(bytes, 128),
            origin_micros: i64_at(bytes, 136),
            sum_off: u64_at(bytes, 144),
            sum_len: u32_at(bytes, 152),
            records_blake3,
            crc_image_crc: u32_at(bytes, 192),
            path_digest,
            columns,
            extremes: RunExtremes {
                close_max: extreme(0),
                close_min: extreme(1),
                high_max: extreme(2),
                low_min: extreme(3),
                range_max: extreme(4),
                volume_max: extreme(5),
            },
        }
    }

    /// The month's IST span, from the head's year and month.
    fn month_bounds(&self) -> Result<(i64, i64), SealedError> {
        YearMonth::new(self.year, self.month)
            .map(YearMonth::ist_bounds_micros)
            .map_err(|_| SealedError::Geometry {
                field: "month",
                head: u64::from(self.month),
                expected: 12,
            })
    }

    /// Every offset and count the head's own counts give, derived again.
    fn check_geometry(&self) -> Result<(), SealedError> {
        let same = |field: &'static str, head: u64, expected: u64| {
            if head == expected {
                Ok(())
            } else {
                Err(SealedError::Geometry {
                    field,
                    head,
                    expected,
                })
            }
        };
        if self.timeframe_secs == 0 {
            return Err(SealedError::Geometry {
                field: "timeframe_secs",
                head: 0,
                expected: 1,
            });
        }
        let (origin, until) = self.month_bounds()?;
        same(
            "origin_micros",
            self.origin_micros.cast_unsigned(),
            origin.cast_unsigned(),
        )?;
        let n_blocks = u64::from(self.n_blocks);
        same(
            "n_blocks",
            n_blocks,
            self.n_valid.div_ceil(u64::from(self.block_rows)),
        )?;
        let table = self.flags & FLAG_TIME_TABLE != 0;
        same(
            "time_table_flag",
            u64::from(table),
            u64::from(n_blocks > TABLE_AFTER_BLOCKS),
        )?;
        let entries = if table {
            table_entries_for(until - origin, self.timeframe_secs)
        } else {
            0
        };
        same("table_entries", u64::from(self.table_entries), entries)?;
        let table_off = HEAD_LEN + ENTRY_LEN * n_blocks;
        same("table_off", self.table_off, table_off)?;
        let table_len = if table {
            (entries + 1) * TABLE_ENTRY_LEN
        } else {
            0
        };
        let prv_off = table_off + table_len;
        same("prv_off", self.prv_off, prv_off)?;
        let packed = u64::from(self.prv_packed_len);
        let raw = u64::from(self.prv_raw_len);
        let prv_len = if self.flags & FLAG_PROVENANCE == 0 {
            same("prv_packed_len", packed, 0)?;
            same("prv_raw_len", raw, 0)?;
            0
        } else {
            same("prv_raw_len", raw, raw.min(MAX_PROVENANCE_LEN))?;
            same("prv_packed_len", packed, packed.min(2 * raw))?;
            packed + 4
        };
        let sum_len = u64::from(self.sum_len);
        let after_prv = prv_off + prv_len;
        let blocks_off = if self.flags & FLAG_SUMMARY == 0 {
            same("sum_off", self.sum_off, 0)?;
            same("sum_len", sum_len, 0)?;
            after_prv
        } else {
            same("sum_off", self.sum_off, after_prv)?;
            let longest = len_u64(MAX_SUMMARY_LEN) + SUMMARY_OVERHEAD;
            same("sum_len", sum_len, sum_len.clamp(SUMMARY_OVERHEAD, longest))?;
            after_prv + sum_len
        };
        same("blocks_off", self.blocks_off, blocks_off)?;
        let most = n_blocks
            .saturating_mul(len_u64(sealed_codec::max_frame_len(self.block_rows)))
            .min(u64::from(u32::MAX));
        let least = n_blocks.min(1);
        same(
            "blocks_len",
            self.blocks_len,
            self.blocks_len.clamp(least, most),
        )?;
        same("file_len", self.file_len, blocks_off + self.blocks_len)
    }

    /// The stamps, the column ranges and the extremes: what a month can hold.
    fn check_contents(&self) -> Result<(), SealedError> {
        let (origin, until) = self.month_bounds()?;
        let (first, last) = (self.first_ts_micros, self.last_ts_micros);
        if self.n_valid > 0 && !(origin <= first && first <= last && last < until) {
            return Err(SealedError::Stamps { first, last });
        }
        for (at, (column, name)) in self.columns.iter().zip(COLUMN_NAMES).enumerate() {
            let nullable = at == 5;
            let bad = (!nullable && column.nulls != 0)
                || column.nulls > self.n_valid
                || if column.nulls == self.n_valid {
                    column.smallest != 0 || column.largest != 0
                } else {
                    column.smallest > column.largest
                };
            if bad {
                return Err(SealedError::Columns { column: name });
            }
        }
        let extremes = extremes_of(&self.extremes);
        if self.n_valid == 0 {
            if let Some(at) = extremes.iter().position(|e| *e != Extreme::default()) {
                return Err(SealedError::Extremes {
                    extreme: EXTREME_NAMES.get(at).copied().unwrap_or("extreme"),
                });
            }
            return Ok(());
        }
        let [_, high, low, close, volume, _] = self.columns;
        // The widest `high - low` has no column range to agree with; it is
        // checked against its own value, so only its stamp is held here.
        let values = [
            close.largest,
            close.smallest,
            high.largest,
            low.smallest,
            self.extremes.range_max.v,
            volume.largest,
        ];
        for ((extreme, value), name) in extremes.iter().zip(values).zip(EXTREME_NAMES) {
            if extreme.v != value || extreme.ts < first || extreme.ts > last {
                return Err(SealedError::Extremes { extreme: name });
            }
        }
        Ok(())
    }

    /// Checks the head against the path it was found at, the symbol id the
    /// caller resolved for that path, and the file's length.
    ///
    /// # Errors
    ///
    /// [`SealedError::Misfiled`] naming `symbol_id`, `timeframe`, `month` or
    /// `path` (the path digest), or [`SealedError::Length`].
    pub fn check(
        &self,
        path: StorePath<'_>,
        symbol_id: u32,
        file_len: u64,
    ) -> Result<(), SealedError> {
        if self.symbol_id != symbol_id {
            return Err(SealedError::Misfiled { field: "symbol_id" });
        }
        if self.timeframe_secs != path.timeframe().secs() {
            return Err(SealedError::Misfiled { field: "timeframe" });
        }
        if self.year != path.month().year() || self.month != path.month().month() {
            return Err(SealedError::Misfiled { field: "month" });
        }
        if self.path_digest != path_digest(path) {
            return Err(SealedError::Misfiled { field: "path" });
        }
        if self.file_len != file_len {
            return Err(SealedError::Length {
                head: self.file_len,
                file: file_len,
            });
        }
        Ok(())
    }

    fn has_table(&self) -> bool {
        self.flags & FLAG_TIME_TABLE != 0
    }
}

fn extremes_of(extremes: &RunExtremes) -> [Extreme; 6] {
    [
        extremes.close_max,
        extremes.close_min,
        extremes.high_max,
        extremes.low_min,
        extremes.range_max,
        extremes.volume_max,
    ]
}

/// The first 24 bytes of BLAKE3 over the month's `.bin` store path: the same
/// function the `.prv` header's digest is (`provenance::Header::of`).
fn path_digest(path: StorePath<'_>) -> [u8; PATH_DIGEST_LEN] {
    provenance::Header::of(path, 0).path_digest
}

// ---------------------------------------------------------------------------
// The fold: one pass over the rows, shared by the builder and the audit
// ---------------------------------------------------------------------------

/// Everything a head states that is a function of the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Folded {
    rows: u64,
    first: i64,
    last: i64,
    digest: [u8; 32],
    crc_image: u32,
    columns: [ColumnRange; 6],
    extremes: RunExtremes,
    table: Vec<u32>,
}

struct Fold {
    origin: i64,
    until: i64,
    slot_micros: i128,
    /// The table's whole length, `entries + 1`: every loop over the table is
    /// bounded by it directly.
    table_rows: usize,
    /// Where the next table entry's superblock starts.
    next_start: i128,
    block_rows: u64,
    rows: u64,
    first: i64,
    last: i64,
    hasher: blake3::Hasher,
    raw_block: Vec<u8>,
    crc_entries: Vec<u8>,
    ranges: [Option<(i64, i64)>; 6],
    oi_nulls: u64,
    extremes: Option<RunExtremes>,
    table: Vec<u32>,
}

/// Bytes of one raw `.crc` block: 73 records.
const RAW_BLOCK: usize = RECORD_LEN * 73;
const _: () = assert!(RAW_BLOCK as u64 == BLOCK_LEN);

impl Fold {
    fn new(origin: i64, until: i64, timeframe_secs: u32, block_rows: u16) -> Self {
        let entries = table_entries_for(until - origin, timeframe_secs);
        let table_rows = usize::try_from(entries).map_or(1, |entries| entries + 1);
        Self {
            origin,
            until,
            slot_micros: i128::from(TABLE_SLOTS) * i128::from(timeframe_secs) * MICROS,
            table_rows,
            next_start: i128::from(origin),
            block_rows: u64::from(block_rows),
            rows: 0,
            first: 0,
            last: 0,
            hasher: blake3::Hasher::new(),
            raw_block: Vec::with_capacity(RAW_BLOCK),
            crc_entries: Vec::new(),
            ranges: [None; 6],
            oi_nulls: 0,
            extremes: None,
            table: Vec::with_capacity(table_rows),
        }
    }

    fn push(&mut self, bar: &Bar) -> Result<(), SealedError> {
        let row = self.rows;
        let ts = bar.ts_micros;
        if row > 0 && ts < self.last {
            return Err(SealedError::OutOfOrder { row });
        }
        if ts < self.origin || ts >= self.until {
            return Err(SealedError::OutsideMonth { row });
        }
        if row == 0 {
            self.first = ts;
        }
        self.last = ts;
        let image = bar.image();
        self.hasher.update(&image);
        self.raw_block.extend_from_slice(&image);
        if self.raw_block.len() == RAW_BLOCK {
            self.crc_entries
                .extend_from_slice(&crc32c(&self.raw_block).to_le_bytes());
            self.raw_block.clear();
        }
        let values = [bar.open, bar.high, bar.low, bar.close, bar.volume];
        for (range, value) in self.ranges.iter_mut().zip(values) {
            widen(range, value);
        }
        if bar.open_interest == OI_NULL {
            self.oi_nulls += 1;
        } else if let Some(range) = self.ranges.get_mut(5) {
            widen(range, bar.open_interest);
        }
        fold_extremes(&mut self.extremes, bar);
        let block = u32::try_from(row / self.block_rows).unwrap_or(u32::MAX);
        while self.table.len() < self.table_rows && self.next_start <= i128::from(ts) {
            self.table.push(block);
            self.next_start += self.slot_micros;
        }
        self.rows += 1;
        Ok(())
    }

    fn finish(self, n_blocks: u32) -> Folded {
        let mut crc_entries = self.crc_entries;
        if !self.raw_block.is_empty() {
            crc_entries.extend_from_slice(&crc32c(&self.raw_block).to_le_bytes());
        }
        let mut table = self.table;
        table.resize(self.table_rows, n_blocks);
        let mut columns = [ColumnRange::default(); 6];
        for (column, range) in columns.iter_mut().zip(self.ranges) {
            let (smallest, largest) = range.unwrap_or((0, 0));
            column.smallest = smallest;
            column.largest = largest;
        }
        if let Some(oi) = columns.get_mut(5) {
            oi.nulls = self.oi_nulls;
        }
        Folded {
            rows: self.rows,
            first: self.first,
            last: self.last,
            digest: self.hasher.finalize(),
            crc_image: crc32c(&crc_entries),
            columns,
            extremes: self.extremes.unwrap_or_default(),
            table,
        }
    }
}

fn widen(range: &mut Option<(i64, i64)>, value: i64) {
    *range = Some(range.map_or((value, value), |(lo, hi)| (lo.min(value), hi.max(value))));
}

/// Folds one bar into the extremes; the earlier bar keeps a tie. The rule
/// `sort_index`'s run header keeps (D-3326), repeated because that fold is
/// private to the sort index; `sealed::the_head_extremes_are_the_sort_runs`
/// holds the two equal.
fn fold_extremes(held: &mut Option<RunExtremes>, bar: &Bar) {
    let at = |v: i64| Extreme {
        v,
        ts: bar.ts_micros,
    };
    let range = bar.high.saturating_sub(bar.low);
    let Some(held) = held.as_mut() else {
        *held = Some(RunExtremes {
            close_max: at(bar.close),
            close_min: at(bar.close),
            high_max: at(bar.high),
            low_min: at(bar.low),
            range_max: at(range),
            volume_max: at(bar.volume),
        });
        return;
    };
    if bar.close > held.close_max.v {
        held.close_max = at(bar.close);
    }
    if bar.close < held.close_min.v {
        held.close_min = at(bar.close);
    }
    if bar.high > held.high_max.v {
        held.high_max = at(bar.high);
    }
    if bar.low < held.low_min.v {
        held.low_min = at(bar.low);
    }
    if range > held.range_max.v {
        held.range_max = at(range);
    }
    if bar.volume > held.volume_max.v {
        held.volume_max = at(bar.volume);
    }
}

// ---------------------------------------------------------------------------
// The builder
// ---------------------------------------------------------------------------

/// Builds a sealed month from a raw month's committed header and records,
/// one record at a time: at most one frame of rows is held unencoded, and
/// the month's frames are held encoded (about 3 to 8 bytes a bar; the
/// largest month, 2,678,400 one-second bars, about 21 MB).
pub struct Builder {
    header: Header,
    prv_header: provenance::Header,
    year: u16,
    month: u8,
    origin: i64,
    pending: Vec<Bar>,
    frames: Vec<u8>,
    directory: Vec<u8>,
    blocks: u64,
    fold: Fold,
}

impl Builder {
    /// A builder for the month at `path`, whose committed header is `header`.
    ///
    /// # Errors
    ///
    /// [`SealedError::UnverifiedSource`] for a version 2 month,
    /// [`SealedError::RawVersion`] for any other version but 3,
    /// [`SealedError::Stride`], [`SealedError::RawFlags`],
    /// [`SealedError::Misfiled`] (`timeframe`) for a header of another rung
    /// than the path's, or [`SealedError::TooLarge`] for more blocks than a
    /// `u32` counts.
    pub fn new(path: StorePath<'_>, header: &Header) -> Result<Self, SealedError> {
        Self::checked(path, header).inspect_err(note_refused)
    }

    fn checked(path: StorePath<'_>, header: &Header) -> Result<Self, SealedError> {
        match header.format_version {
            SEALED_FROM_VERSION => {}
            2 => return Err(SealedError::UnverifiedSource),
            found => return Err(SealedError::RawVersion { found }),
        }
        if header.record_stride != STRIDE {
            return Err(SealedError::Stride {
                found: header.record_stride,
            });
        }
        if header.flags != FLAG_CHECKSUMS {
            return Err(SealedError::RawFlags {
                found: header.flags,
            });
        }
        if header.timeframe_secs != path.timeframe().secs() {
            return Err(SealedError::Misfiled { field: "timeframe" });
        }
        let blocks = header.n_valid.div_ceil(u64::from(BLOCK_ROWS));
        if u32::try_from(blocks).is_err() {
            return Err(SealedError::TooLarge);
        }
        let (origin, until) = path.month().ist_bounds_micros();
        Ok(Self {
            header: *header,
            prv_header: provenance::Header::of(path, header.symbol_id),
            year: path.month().year(),
            month: path.month().month(),
            origin,
            pending: Vec::with_capacity(usize::from(BLOCK_ROWS)),
            frames: Vec::new(),
            directory: Vec::with_capacity(usize::try_from(blocks.min(16_384)).unwrap_or(0) * ENTRY),
            blocks: 0,
            fold: Fold::new(origin, until, header.timeframe_secs, BLOCK_ROWS),
        })
    }

    /// Offers the next committed record.
    ///
    /// # Errors
    ///
    /// [`SealedError::TooManyRows`] past the header's `n_valid`,
    /// [`SealedError::OutOfOrder`] for a stamp before the one before it,
    /// [`SealedError::OutsideMonth`] for a stamp outside the month's IST span,
    /// or [`SealedError::TooLarge`] for frames past a `u32` of bytes.
    pub fn push(&mut self, bar: &Bar) -> Result<(), SealedError> {
        self.pushed(bar).inspect_err(note_refused)
    }

    fn pushed(&mut self, bar: &Bar) -> Result<(), SealedError> {
        if self.fold.rows >= self.header.n_valid {
            return Err(SealedError::TooManyRows {
                n_valid: self.header.n_valid,
            });
        }
        self.fold.push(bar)?;
        self.pending.push(*bar);
        if self.pending.len() == usize::from(BLOCK_ROWS) {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), SealedError> {
        let Some(first) = self.pending.first().map(|bar| bar.ts_micros) else {
            return Ok(());
        };
        // `pending` holds 1 to `BLOCK_ROWS` rows here, which the codec always
        // encodes; its refusal is kept a value rather than a panic.
        let encoded =
            sealed_codec::encode_frame(&self.pending, self.origin, BLOCK_ROWS, &mut self.frames)
                .ok()
                .ok_or(SealedError::TooLarge)?;
        let end = u32::try_from(self.frames.len()).map_err(|_| SealedError::TooLarge)?;
        self.directory
            .extend_from_slice(&entry_image(self.blocks, end, encoded.crc, first));
        self.blocks += 1;
        self.pending.clear();
        Ok(())
    }

    /// Finishes the month: the frames, the directory, the time table when
    /// the month has more than 64 blocks, the packed provenance and the
    /// summary, under a head that states them.
    ///
    /// `provenance` is the month's `.prv` image byte for byte, or `None` when
    /// the month has none. An image of 64 bytes or more must carry this
    /// month's `.prv` header; a shorter one (a sidecar whose header write was
    /// cut) is carried as it is, which is what `provenance::read` reports of
    /// it. `summary` is R3-06's month summary, carried and never read.
    ///
    /// # Errors
    ///
    /// [`SealedError::Count`] for fewer rows than the header commits,
    /// [`SealedError::HeaderStamps`] for rows that do not start and end at
    /// its stamps, [`SealedError::TooLarge`], [`SealedError::ProvenanceTooLong`],
    /// [`SealedError::ProvenanceForeign`], [`SealedError::SummaryTooLong`] or
    /// [`SealedError::SummaryVersion`].
    pub fn finish(
        self,
        provenance: Option<&[u8]>,
        summary: Option<Summary<'_>>,
    ) -> Result<Built, SealedError> {
        self.finished(provenance, summary).inspect_err(note_refused)
    }

    fn finished(
        mut self,
        provenance: Option<&[u8]>,
        summary: Option<Summary<'_>>,
    ) -> Result<Built, SealedError> {
        let header = self.header;
        if self.fold.rows != header.n_valid {
            return Err(SealedError::Count {
                expected: header.n_valid,
                found: self.fold.rows,
            });
        }
        self.flush()?;
        if header.n_valid > 0
            && (self.fold.first != header.first_ts_micros
                || self.fold.last != header.last_ts_micros)
        {
            return Err(SealedError::HeaderStamps);
        }
        let prv = self.packed_provenance(provenance)?;
        let sum = Self::summary_section(summary)?;
        let n_blocks = u32::try_from(self.blocks).map_err(|_| SealedError::TooLarge)?;
        let folded = self.fold.finish(n_blocks);
        let table = u64::from(n_blocks) > TABLE_AFTER_BLOCKS;

        let mut tables = self.directory;
        let mut flags = 0;
        let mut table_entries = 0;
        if table {
            flags |= FLAG_TIME_TABLE;
            table_entries =
                u32::try_from(folded.table.len() - 1).map_err(|_| SealedError::TooLarge)?;
            for (index, block) in (0u64..).zip(&folded.table) {
                tables.extend_from_slice(&table_image(index, *block));
            }
        }
        let prv_off = HEAD_LEN + len_u64(tables.len());
        let (mut prv_packed_len, mut prv_raw_len) = (0, 0);
        if let Some((packed, raw_len)) = prv {
            flags |= FLAG_PROVENANCE;
            prv_packed_len = u32::try_from(packed.len()).map_err(|_| SealedError::TooLarge)?;
            prv_raw_len = raw_len;
            tables.extend_from_slice(&packed);
            tables.extend_from_slice(&crc32c(&packed).to_le_bytes());
        }
        let (mut sum_off, mut sum_len) = (0, 0);
        if let Some(section) = sum {
            flags |= FLAG_SUMMARY;
            sum_off = HEAD_LEN + len_u64(tables.len());
            sum_len = u32::try_from(section.len()).map_err(|_| SealedError::TooLarge)?;
            tables.extend_from_slice(&section);
        }
        let blocks_off = HEAD_LEN + len_u64(tables.len());
        let blocks_len = len_u64(self.frames.len());
        let head = Head {
            flags,
            generation: header.generation,
            n_valid: header.n_valid,
            first_ts_micros: header.first_ts_micros,
            last_ts_micros: header.last_ts_micros,
            symbol_id: header.symbol_id,
            timeframe_secs: header.timeframe_secs,
            block_rows: BLOCK_ROWS,
            raw_version: header.format_version,
            n_blocks,
            table_entries,
            prv_packed_len,
            prv_raw_len,
            year: self.year,
            month: self.month,
            table_off: HEAD_LEN + ENTRY_LEN * u64::from(n_blocks),
            prv_off,
            blocks_off,
            blocks_len,
            file_len: blocks_off + blocks_len,
            origin_micros: self.origin,
            sum_off,
            sum_len,
            records_blake3: folded.digest,
            crc_image_crc: folded.crc_image,
            path_digest: self.prv_header.path_digest,
            columns: folded.columns,
            extremes: folded.extremes,
        };
        Ok(Built {
            image: head.encode(),
            head,
            tables,
            frames: self.frames,
        })
    }

    fn packed_provenance(
        &self,
        provenance: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, u32)>, SealedError> {
        let Some(image) = provenance else {
            return Ok(None);
        };
        let len = len_u64(image.len());
        if len > MAX_PROVENANCE_LEN {
            return Err(SealedError::ProvenanceTooLong { len });
        }
        let foreign = image
            .first_chunk::<64>()
            .is_some_and(|head| provenance::Header::decode(head).ok() != Some(self.prv_header));
        if foreign {
            return Err(SealedError::ProvenanceForeign);
        }
        let raw_len = u32::try_from(len).map_err(|_| SealedError::ProvenanceTooLong { len })?;
        Ok(Some((pack_provenance(image), raw_len)))
    }

    fn summary_section(summary: Option<Summary<'_>>) -> Result<Option<Vec<u8>>, SealedError> {
        let Some(summary) = summary else {
            return Ok(None);
        };
        if summary.bytes.len() > MAX_SUMMARY_LEN {
            return Err(SealedError::SummaryTooLong {
                len: len_u64(summary.bytes.len()),
            });
        }
        if summary.version == 0 {
            return Err(SealedError::SummaryVersion);
        }
        let mut section = Vec::with_capacity(summary.bytes.len() + 8);
        section.extend_from_slice(&summary.version.to_le_bytes());
        section.extend_from_slice(&[0, 0]);
        section.extend_from_slice(summary.bytes);
        let crc = crc32c(&section);
        section.extend_from_slice(&crc.to_le_bytes());
        Ok(Some(section))
    }
}

/// Builds a sealed month from all its rows at once: [`Builder`], one call.
///
/// # Errors
///
/// What [`Builder::new`], [`Builder::push`] and [`Builder::finish`] refuse.
pub fn build(
    path: StorePath<'_>,
    header: &Header,
    bars: &[Bar],
    provenance: Option<&[u8]>,
    summary: Option<Summary<'_>>,
) -> Result<Built, SealedError> {
    let mut builder = Builder::new(path, header)?;
    for bar in bars {
        builder.push(bar)?;
    }
    builder.finish(provenance, summary)
}

/// A built sealed month, ready to be written whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    head: Head,
    image: [u8; HEAD],
    tables: Vec<u8>,
    frames: Vec<u8>,
}

impl Built {
    /// The head it carries.
    #[must_use]
    pub const fn head(&self) -> &Head {
        &self.head
    }

    /// The file's length in bytes.
    #[must_use]
    pub const fn file_len(&self) -> u64 {
        self.head.file_len
    }

    /// Writes the file's bytes, in order, to `out`: head, tables, frames.
    ///
    /// # Errors
    ///
    /// The writer's refusal.
    pub fn write_to<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(&self.image)?;
        out.write_all(&self.tables)?;
        out.write_all(&self.frames)
    }

    /// The file's bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEAD + self.tables.len() + self.frames.len());
        out.extend_from_slice(&self.image);
        out.extend_from_slice(&self.tables);
        out.extend_from_slice(&self.frames);
        out
    }
}

// ---------------------------------------------------------------------------
// The reader
// ---------------------------------------------------------------------------

/// Something a sealed month can be read from by position: a [`File`], or
/// bytes held in memory.
pub trait Positional {
    /// Fills `buf` from `offset`, or refuses.
    ///
    /// # Errors
    ///
    /// The host's refusal, or [`io::ErrorKind::UnexpectedEof`] past the end.
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>;

    /// Bytes it holds.
    ///
    /// # Errors
    ///
    /// The host's refusal to measure it.
    fn byte_len(&self) -> io::Result<u64>;
}

impl Positional for File {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        FileExt::read_exact_at(self, buf, offset)
    }

    fn byte_len(&self) -> io::Result<u64> {
        self.metadata().map(|meta| meta.len())
    }
}

impl Positional for [u8] {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        let src = usize::try_from(offset)
            .ok()
            .and_then(|start| self.get(start..start.checked_add(buf.len())?))
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        buf.copy_from_slice(src);
        Ok(())
    }

    fn byte_len(&self) -> io::Result<u64> {
        Ok(len_u64(self.len()))
    }
}

impl Positional for Vec<u8> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.as_slice().read_exact_at(buf, offset)
    }

    fn byte_len(&self) -> io::Result<u64> {
        self.as_slice().byte_len()
    }
}

impl<T: Positional + ?Sized> Positional for &T {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        (**self).read_exact_at(buf, offset)
    }

    fn byte_len(&self) -> io::Result<u64> {
        (**self).byte_len()
    }
}

/// What [`SealedMonth::audit`] read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audited {
    /// Rows decoded.
    pub rows: u64,
    /// Frames decoded.
    pub frames: u64,
    /// Bytes the file holds.
    pub bytes: u64,
}

/// An open sealed month: its checked head, and the source it reads from.
/// No lock is taken: the file is published once by rename and never written.
#[derive(Debug)]
pub struct SealedMonth<R = File> {
    source: R,
    head: Head,
}

impl<R: Positional> SealedMonth<R> {
    /// Opens the sealed month at `path` held by `source`: reads its length
    /// and its head, and checks the head ([`Head::decode`], [`Head::check`]).
    ///
    /// # Errors
    ///
    /// [`SealedError::Io`], or what [`Head::decode`] and [`Head::check`]
    /// refuse.
    pub fn open(source: R, path: StorePath<'_>, symbol_id: u32) -> Result<Self, SealedError> {
        Self::opened(source, path, symbol_id).inspect_err(note_refused)
    }

    fn opened(source: R, path: StorePath<'_>, symbol_id: u32) -> Result<Self, SealedError> {
        let len = source.byte_len().map_err(|source| SealedError::Io {
            action: "measuring the sealed month",
            offset: 0,
            source,
        })?;
        if len < HEAD_LEN {
            return Err(SealedError::TooShort { len });
        }
        let mut bytes = [0u8; HEAD];
        source
            .read_exact_at(&mut bytes, 0)
            .map_err(|source| SealedError::Io {
                action: "reading the head",
                offset: 0,
                source,
            })?;
        let head = Head::decode(&bytes)?;
        head.check(path, symbol_id, len)?;
        Ok(Self { source, head })
    }

    /// The checked head: the month's counts, stamps, ranges and extremes,
    /// answered with no read beyond the open.
    #[must_use]
    pub const fn head(&self) -> &Head {
        &self.head
    }

    fn read(&self, action: &'static str, offset: u64, len: u64) -> Result<Vec<u8>, SealedError> {
        let mut buf = vec![0u8; usize::try_from(len).map_err(|_| SealedError::TooLarge)?];
        self.source
            .read_exact_at(&mut buf, offset)
            .map_err(|source| SealedError::Io {
                action,
                offset,
                source,
            })?;
        Ok(buf)
    }

    /// Directory entries `from .. from + count`, in one read, each checked.
    fn entries(&self, from: u64, count: u64) -> Result<Vec<Entry>, SealedError> {
        let bytes = self.read(
            "reading directory entries",
            HEAD_LEN + ENTRY_LEN * from,
            ENTRY_LEN * count,
        )?;
        bytes
            .chunks_exact(ENTRY)
            .zip(from..)
            .map(|(entry, index)| Entry::decode(entry, index))
            .collect()
    }

    /// Time-table entries `from .. from + count`, in one read, each checked.
    fn table(&self, from: u64, count: u64) -> Result<Vec<u32>, SealedError> {
        let bytes = self.read(
            "reading the time table",
            self.head.table_off + TABLE_ENTRY_LEN * from,
            TABLE_ENTRY_LEN * count,
        )?;
        let mut out = Vec::with_capacity(bytes.len() / TABLE_ENTRY);
        for (entry, index) in bytes.chunks_exact(TABLE_ENTRY).zip(from..) {
            let block = u32_at(entry, 0);
            if u32_at(entry, 4) != crc32c_split(&block.to_le_bytes(), &index.to_le_bytes()) {
                return Err(SealedError::Table { index });
            }
            let before = out.last().copied().unwrap_or(0);
            if block > self.head.n_blocks || block < before {
                return Err(SealedError::TableOrder { index });
            }
            out.push(block);
        }
        Ok(out)
    }

    /// Checks that frame `block` spans `start .. entry.end` sanely.
    fn span(&self, block: u64, start: u32, end: u32) -> Result<(), SealedError> {
        let last = u64::from(self.head.n_blocks).saturating_sub(1);
        let bad = end < start
            || u64::from(end) > self.head.blocks_len
            || len_u64(sealed_codec::max_frame_len(self.head.block_rows)) < u64::from(end - start)
            || (block == last && u64::from(end) != self.head.blocks_len);
        if bad {
            Err(SealedError::EntryOrder { block })
        } else {
            Ok(())
        }
    }

    /// Decodes frame `block` from `bytes` and appends its rows to `out`;
    /// on a refusal `out` is as it was.
    fn decode(
        &self,
        block: u64,
        bytes: &[u8],
        entry: Entry,
        out: &mut Vec<Bar>,
    ) -> Result<(), SealedError> {
        let start = out.len();
        let rows = sealed_codec::decode_frame(
            bytes,
            entry.crc,
            self.head.origin_micros,
            self.head.block_rows,
            out,
        )
        .map_err(|why| SealedError::Frame { block, why })?;
        let expected = self.head.rows_in(block);
        let refusal = if len_u64(rows) != expected {
            Some(SealedError::FrameRows {
                block,
                expected,
                found: len_u64(rows),
            })
        } else if out.get(start).map(|bar| bar.ts_micros) != Some(entry.first_ts) {
            Some(SealedError::FirstStamp { block })
        } else {
            None
        };
        if let Some(refusal) = refusal {
            out.truncate(start);
            return Err(refusal);
        }
        Ok(())
    }

    /// Frames `first ..= last`: one read of their directory entries (and the
    /// one before, for where the first starts), one read of their bytes.
    fn frames(&self, first: u64, last: u64, out: &mut Vec<Bar>) -> Result<(), SealedError> {
        let from = first.saturating_sub(1);
        let entries = self.entries(from, last - from + 1)?;
        let lead = usize::from(first > 0);
        let mut start = if first > 0 {
            entries.first().map_or(0, |entry| entry.end)
        } else {
            0
        };
        let region_start = start;
        let mine = entries.get(lead..).unwrap_or_default();
        for (entry, block) in mine.iter().zip(first..) {
            self.span(block, start, entry.end)?;
            start = entry.end;
        }
        let bytes = self.read(
            "reading frames",
            self.head.blocks_off + u64::from(region_start),
            u64::from(start - region_start),
        )?;
        let at = out.len();
        let mut from_byte = region_start;
        for (entry, block) in mine.iter().zip(first..) {
            let span = usize::try_from(from_byte - region_start).unwrap_or(usize::MAX)
                ..usize::try_from(entry.end - region_start).unwrap_or(usize::MAX);
            if let Err(refusal) =
                self.decode(block, bytes.get(span).unwrap_or_default(), *entry, out)
            {
                out.truncate(at);
                return Err(refusal);
            }
            from_byte = entry.end;
        }
        Ok(())
    }

    /// Appends the rows of block `block` to `out`; returns how many.
    ///
    /// # Errors
    ///
    /// [`SealedError::Range`] past the month's blocks; an entry, span or
    /// frame refusal naming the block; [`SealedError::Io`]. On an error `out`
    /// is as it was.
    pub fn read_block(&self, block: u64, out: &mut Vec<Bar>) -> Result<usize, SealedError> {
        self.block_read(block, out).inspect_err(note_refused)
    }

    fn block_read(&self, block: u64, out: &mut Vec<Bar>) -> Result<usize, SealedError> {
        if block >= u64::from(self.head.n_blocks) {
            return Err(SealedError::Range {
                first: block.saturating_mul(u64::from(self.head.block_rows)),
                count: u64::from(self.head.block_rows),
                n_valid: self.head.n_valid,
            });
        }
        let at = out.len();
        self.frames(block, block, out)?;
        Ok(out.len() - at)
    }

    /// Appends rows `first .. first + count` to `out`; returns `count`. At
    /// most two more frames are read than the rows fill.
    ///
    /// # Errors
    ///
    /// [`SealedError::Range`] for rows past the month or more than
    /// [`MAX_RANGE_ROWS`]; an entry, span or frame refusal naming its block;
    /// [`SealedError::Io`]. On an error `out` is as it was.
    pub fn read_rows(
        &self,
        first: u64,
        count: u64,
        out: &mut Vec<Bar>,
    ) -> Result<usize, SealedError> {
        self.rows_read(first, count, out).inspect_err(note_refused)
    }

    fn rows_read(&self, first: u64, count: u64, out: &mut Vec<Bar>) -> Result<usize, SealedError> {
        let n_valid = self.head.n_valid;
        let past = first.checked_add(count).is_none_or(|end| end > n_valid);
        if count > MAX_RANGE_ROWS || past {
            return Err(SealedError::Range {
                first,
                count,
                n_valid,
            });
        }
        if count == 0 {
            return Ok(0);
        }
        let rows = u64::from(self.head.block_rows);
        let (k0, k1) = (first / rows, (first + count - 1) / rows);
        let mut held = Vec::with_capacity(usize::try_from((k1 - k0 + 1) * rows).unwrap_or(0));
        self.frames(k0, k1, &mut held)?;
        let skip = usize::try_from(first - k0 * rows).unwrap_or(usize::MAX);
        let wanted = usize::try_from(count).unwrap_or(usize::MAX);
        out.extend_from_slice(held.get(skip..skip + wanted).unwrap_or_default());
        Ok(wanted)
    }

    /// The first row stamped at or after `ts`, or `n_valid` when none is.
    ///
    /// A stamp at or before the first row's answers 0, and one after the
    /// last row's answers `n_valid`, from the head. Otherwise: with a time
    /// table, one read of two table entries names the at most 17 blocks the
    /// stamp's 4,096 slots span; without one, the month has at most 64
    /// blocks. One read of their directory entries, a bisection by first
    /// stamp, and one frame's stamps decoded and bisected. Equal stamps
    /// answer the first of them.
    ///
    /// # Errors
    ///
    /// An entry, table, span or frame refusal; [`SealedError::Io`].
    pub fn first_at_or_after(&self, ts: i64) -> Result<u64, SealedError> {
        self.lookup(ts).inspect_err(note_refused)
    }

    fn lookup(&self, ts: i64) -> Result<u64, SealedError> {
        let head = &self.head;
        if head.n_valid == 0 || ts <= head.first_ts_micros {
            return Ok(0);
        }
        if ts > head.last_ts_micros {
            return Ok(head.n_valid);
        }
        let last = u64::from(head.n_blocks) - 1;
        let (lo, hi) = if head.has_table() {
            let slot = i128::from(TABLE_SLOTS) * i128::from(head.timeframe_secs) * MICROS;
            let s = (i128::from(ts) - i128::from(head.origin_micros)) / slot;
            let s = u64::try_from(s)
                .unwrap_or(0)
                .min(u64::from(head.table_entries).saturating_sub(1));
            let pair = self.table(s, 2)?;
            let (lo, hi) = (
                pair.first().copied().map_or(0, u64::from),
                pair.get(1).copied().map_or(0, u64::from),
            );
            if lo > last {
                return Err(SealedError::TableOrder { index: s });
            }
            (lo, hi.min(last))
        } else {
            (0, last)
        };
        let Some((block, start, entry)) = self.last_before(lo, hi, ts)? else {
            return Ok(lo * u64::from(head.block_rows));
        };
        self.span(block, start, entry.end)?;
        let bytes = self.read(
            "reading a frame's stamps",
            head.blocks_off + u64::from(start),
            u64::from(entry.end - start),
        )?;
        let mut stamps = Vec::with_capacity(usize::from(head.block_rows));
        sealed_codec::decode_stamps(
            &bytes,
            entry.crc,
            head.origin_micros,
            head.block_rows,
            &mut stamps,
        )
        .map_err(|why| SealedError::Frame { block, why })?;
        if len_u64(stamps.len()) != head.rows_in(block) {
            return Err(SealedError::FrameRows {
                block,
                expected: head.rows_in(block),
                found: len_u64(stamps.len()),
            });
        }
        if stamps.first() != Some(&entry.first_ts) {
            return Err(SealedError::FirstStamp { block });
        }
        let within = stamps.partition_point(|&stamp| stamp < ts);
        Ok(block * u64::from(head.block_rows) + len_u64(within))
    }

    /// The last block in `lo ..= hi` whose first stamp is before `ts`, with
    /// where its frame starts and its entry; `None` when block `lo`'s first
    /// stamp is at or after `ts`.
    fn last_before(
        &self,
        lo: u64,
        hi: u64,
        ts: i64,
    ) -> Result<Option<(u64, u32, Entry)>, SealedError> {
        if hi - lo < BISECT_IN_ONE_READ {
            let from = lo.saturating_sub(1);
            let entries = self.entries(from, hi - from + 1)?;
            let skip = usize::try_from(lo - from).unwrap_or(usize::MAX);
            let window = entries.get(skip..).unwrap_or_default();
            let before = window.partition_point(|entry| entry.first_ts < ts);
            let Some(at) = before.checked_sub(1) else {
                return Ok(None);
            };
            let index = skip + at;
            let block = lo + len_u64(at);
            let start = match index.checked_sub(1) {
                Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
                _ => 0,
            };
            return Ok(entries.get(index).map(|entry| (block, start, *entry)));
        }
        // A widened range: bisect with one entry read a step,
        // ceil(log2(hi - lo + 2)) reads.
        let (mut a, mut b) = (lo, hi + 1);
        while a < b {
            let mid = a + (b - a) / 2;
            let entry = self.entries(mid, 1)?;
            if entry.first().is_some_and(|entry| entry.first_ts < ts) {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        let Some(block) = a.checked_sub(1).filter(|&k| k >= lo) else {
            return Ok(None);
        };
        let from = block.saturating_sub(1);
        let pair = self.entries(from, block - from + 1)?;
        let start = if block > 0 {
            pair.first().map_or(0, |entry| entry.end)
        } else {
            0
        };
        Ok(pair.last().map(|entry| (block, start, *entry)))
    }

    /// The month's `.prv` image, unpacked and checked, or `None` when the
    /// month was sealed without one.
    ///
    /// # Errors
    ///
    /// [`SealedError::Provenance`] (`checksum`, `truncated`, `overrun`,
    /// `short`), [`SealedError::Io`].
    pub fn provenance(&self) -> Result<Option<Vec<u8>>, SealedError> {
        self.provenance_read().inspect_err(note_refused)
    }

    fn provenance_read(&self) -> Result<Option<Vec<u8>>, SealedError> {
        if self.head.flags & FLAG_PROVENANCE == 0 {
            return Ok(None);
        }
        let packed_len = u64::from(self.head.prv_packed_len);
        let section = self.read("reading the provenance", self.head.prv_off, packed_len + 4)?;
        let (packed, crc) = section.split_at(section.len() - 4);
        if crc32c(packed).to_le_bytes() != take::<4>(crc, 0) {
            return Err(SealedError::Provenance { why: "checksum" });
        }
        let raw_len = usize::try_from(self.head.prv_raw_len).unwrap_or(usize::MAX);
        unpack_provenance(packed, raw_len).map(Some)
    }

    /// The month summary's version and bytes, checked, or `None` when the
    /// month was sealed without one.
    ///
    /// # Errors
    ///
    /// [`SealedError::Summary`] (`checksum`, `version`, `reserved`),
    /// [`SealedError::Io`].
    pub fn summary(&self) -> Result<Option<(u16, Vec<u8>)>, SealedError> {
        self.summary_read().inspect_err(note_refused)
    }

    fn summary_read(&self) -> Result<Option<(u16, Vec<u8>)>, SealedError> {
        if self.head.flags & FLAG_SUMMARY == 0 {
            return Ok(None);
        }
        let section = self.read(
            "reading the month summary",
            self.head.sum_off,
            u64::from(self.head.sum_len),
        )?;
        let (body, crc) = section.split_at(section.len() - 4);
        if crc32c(body).to_le_bytes() != take::<4>(crc, 0) {
            return Err(SealedError::Summary { why: "checksum" });
        }
        let version = u16_at(body, 0);
        if version == 0 {
            return Err(SealedError::Summary { why: "version" });
        }
        if u16_at(body, 2) != 0 {
            return Err(SealedError::Summary { why: "reserved" });
        }
        Ok(Some((version, body.get(4..).unwrap_or_default().to_vec())))
    }

    /// Reads and decodes the whole month and checks it against its head:
    /// every directory entry and frame, the stamps' order, the BLAKE3 of the
    /// records, the regenerated `.crc` image, the column ranges, the
    /// extremes, the time table, the provenance and the summary. O(the
    /// month), once (`docs/06-limits.md` L-SC-16): what a seal verifies and
    /// what a scrub re-verifies, never a request.
    ///
    /// # Errors
    ///
    /// The first refusal: an entry, span or frame naming its block,
    /// [`SealedError::OutOfOrder`] or [`SealedError::OutsideMonth`] naming
    /// its row, [`SealedError::Audit`] naming the head field that differs,
    /// or a table, provenance or summary refusal.
    pub fn audit(&self) -> Result<Audited, SealedError> {
        self.audited().inspect_err(note_refused)
    }

    fn audited(&self) -> Result<Audited, SealedError> {
        let head = &self.head;
        let (origin, until) = head.month_bounds()?;
        let mut fold = Fold::new(origin, until, head.timeframe_secs, head.block_rows);
        let n_blocks = u64::from(head.n_blocks);
        let mut start = 0u32;
        let mut rows = Vec::with_capacity(usize::from(head.block_rows));
        let mut chunk = 0;
        while chunk < n_blocks {
            let count = (n_blocks - chunk).min(256);
            for (entry, block) in self.entries(chunk, count)?.into_iter().zip(chunk..) {
                self.span(block, start, entry.end)?;
                let bytes = self.read(
                    "reading a frame",
                    head.blocks_off + u64::from(start),
                    u64::from(entry.end - start),
                )?;
                rows.clear();
                self.decode(block, &bytes, entry, &mut rows)?;
                for bar in &rows {
                    fold.push(bar)?;
                }
                start = entry.end;
            }
            chunk += count;
        }
        let folded = fold.finish(head.n_blocks);
        let checks: [(&'static str, bool); 6] = [
            (
                "first_ts_micros",
                head.n_valid == 0 || folded.first == head.first_ts_micros,
            ),
            (
                "last_ts_micros",
                head.n_valid == 0 || folded.last == head.last_ts_micros,
            ),
            ("records_blake3", folded.digest == head.records_blake3),
            ("crc_image_crc", folded.crc_image == head.crc_image_crc),
            ("columns", folded.columns == head.columns),
            ("extremes", folded.extremes == head.extremes),
        ];
        if let Some((field, _)) = checks.iter().find(|(_, holds)| !holds) {
            return Err(SealedError::Audit { field });
        }
        if head.has_table() {
            let stored = self.table(0, u64::from(head.table_entries) + 1)?;
            if stored != folded.table {
                return Err(SealedError::Audit {
                    field: "time_table",
                });
            }
        }
        self.provenance_read()?;
        self.summary_read()?;
        Ok(Audited {
            rows: folded.rows,
            frames: n_blocks,
            bytes: head.file_len,
        })
    }
}

#[cfg(test)]
#[path = "sealed_tests.rs"]
mod sealed_tests;

/// Mutation-survivor and coverage tests for this module's private items
/// (cloud session 2026-10-16, report `reports/cloud-mut-store-20261016.md`).
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod cloud_mutant_tests {
    use super::{TABLE_SLOTS, table_entries_for};

    /// **A MONTH'S TIME TABLE HAS ONE ENTRY PER `TABLE_SLOTS` BARS OF ITS
    /// SPAN, ROUNDED UP, AT EVERY TIMEFRAME.**
    ///
    /// Kills `replace * with /` and `replace * with +` in the slot width and
    /// `replace - with +` and `replace - with /` in the rounding of
    /// `table_entries_for`. Only a month of more than `TABLE_AFTER_BLOCKS`
    /// blocks has a table, so the suite meets this function only in its heavy
    /// tests, at one second, where `4096 / 1` equals `4096 * 1`; here every
    /// edge of the rounding is pinned at three timeframes: an empty span, one
    /// microsecond, exactly one slot, one microsecond past it, and a whole
    /// 31-day month.
    #[test]
    fn a_time_table_has_one_entry_per_slot_of_the_span_rounded_up() {
        for timeframe in [1u32, 60, 86_400] {
            let slot = i64::from(TABLE_SLOTS) * i64::from(timeframe) * 1_000_000;
            for (span, entries) in [
                (0, 0),
                (1, 1),
                (slot - 1, 1),
                (slot, 1),
                (slot + 1, 2),
                (3 * slot, 3),
            ] {
                assert_eq!(
                    table_entries_for(span, timeframe),
                    entries,
                    "{span} us at {timeframe} s"
                );
            }
        }
        // 2,678,400 one-second bars over slots of 4,096: 653.9, so 654.
        assert_eq!(table_entries_for(31 * 86_400 * 1_000_000, 1), 654);
        assert_eq!(table_entries_for(31 * 86_400 * 1_000_000, 60), 11);
    }
}
