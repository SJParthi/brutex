//! Where a month's bars came from: the `.prv` provenance sidecar, version 1.
//!
//! `docs/02-store-format.md` §8.3 is the authority for every byte here, and
//! D-3334 is the decision. Findings DB-078 (no row can be audited back to its
//! source) and DB-G56 (nothing in the store tells a zip-built month from a
//! tick-store-built one, although the two sources differ).
//!
//! # What it is
//!
//! A sibling of a month's `.bin`, `<yyyy-mm>.prv`, holding one fixed-size
//! record per **landing** — one `BarFile::append` that committed bars to the
//! month. A record names:
//!
//! - the feed (`vendor`);
//! - the source: its kind (`archive_member`, `tickstore_day` or
//!   `vendor_endpoint`), its name (an archive member, a tick-store day file or
//!   an endpoint path), its size, the archive directory's CRC-32 for a member,
//!   and the BLAKE3 of the whole name;
//! - the IST day the journal keys the landing by;
//! - which bars the landing appended (`first_index`, `n_bars`) and the bar
//!   file's commit generation that published them;
//! - the bar definition that built them (D-2822);
//! - the journal record the landing wrote (`imports/gdfl.journal` line or
//!   `audit/pull.journal` record);
//! - the build commit, or the explicit statement that the build carried none;
//! - when the record was written.
//!
//! # What it never does
//!
//! **It never touches a bar file.** It opens nothing but the `.prv`; the
//! `.bin`, `.crc`, `.tix` and `.lock` are byte-identical before and after a
//! record is written, which `store::provenance::the_bar_files_are_byte_identical_after_a_record`
//! (`crates/store/tests/provenance.rs`) proves by digest.
//!
//! **It never claims a bar that is not committed.** [`record`] takes the
//! writer's own [`BarFile`] and the [`Appended`] its last append returned, and
//! refuses a landing that is not that handle's newest commit. So a record is
//! written strictly *after* its bars, and a crash between the two leaves bars
//! with no record — a gap the reader reports as unrecorded — never a record
//! with no bars.
//!
//! **It never rewrites a record.** Records are appended at
//! `64 + 256 · ordinal`, each sealed by a CRC-32C bound to its ordinal. The
//! only bytes an append may overwrite are the ones past the last valid record:
//! an interrupted append, which no reader ever served (the same rule a bar
//! file's next append applies to the bytes past its commit counter, D-0189).
//!
//! # Reading is a path probe
//!
//! [`read`] is one `open` of one path, one `fstat` and one positional read of
//! at most [`MAX_FILE_LEN`] bytes; [`Sidecar::covering`] then answers which
//! record holds a bar by a binary search over at most [`MAX_RECORDS`]
//! records. Both are bounded by constants that do not depend on the store,
//! the month's bar count or the number of months: `CLAUDE.md` §3 rule 4.
//!
//! # What it does not prove
//!
//! The CRC-32C is integrity, not authentication: a hand-edited record with a
//! recomputed checksum is read as written. The record says what its writer
//! stated about the source; it does not re-read the archive or the endpoint.
//! A month written before D-3334 has no sidecar, and its bars are
//! unrecorded until a later landing — `/provenance.json` (WP-38b) rebuilds
//! those from the journals and labels the answer reconstructed.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;

use crate::crc::{crc32c, crc32c_split};
use crate::file::{Appended, BarFile};
use crate::flock::Flock;
use crate::path::{FileKind, StorePath};

/// The magic at byte 0 of every `.prv`.
pub const MAGIC: [u8; 8] = *b"BRUTEXP1";

/// The one version this build reads and writes. A change to any byte below
/// is a new version, never a reinterpretation of this one (`CLAUDE.md` §3
/// rule 8).
pub const VERSION: u16 = 1;

/// Bytes of the header at offset 0.
pub const HEADER_LEN: u64 = 64;

/// Bytes of one record.
pub const RECORD_STRIDE: u64 = 256;

/// The most records one month's sidecar holds. A landing past it is refused
/// by name ([`ProvenanceError::Full`]).
///
/// A daily import or pull lands once per trading day, so a month holds about
/// 21 records and at most 31; this is thirty-odd times that. It is what
/// bounds a read: [`MAX_FILE_LEN`] is 262,208 bytes, whatever the store holds.
pub const MAX_RECORDS: u64 = 1024;

/// The longest a `.prv` may be: the header and [`MAX_RECORDS`] records. A
/// longer file is refused ([`ProvenanceError::TooLarge`]) before it is read.
pub const MAX_FILE_LEN: u64 = HEADER_LEN + MAX_RECORDS * RECORD_STRIDE;

/// Bytes of a source name a record keeps. A longer name is cut at the last
/// character boundary that fits, the record says so, and the BLAKE3 of the
/// whole name still identifies it exactly.
pub const NAME_CAP: usize = 128;

/// `flags` bit 0: the build that wrote the record carried a commit stamp.
const FLAG_COMMIT: u8 = 1;
/// `flags` bit 1: `source_name` holds a prefix of the name, not all of it.
const FLAG_TRUNCATED: u8 = 2;
/// Every `flags` bit version 1 defines. Written as one literal rather than
/// `FLAG_COMMIT | FLAG_TRUNCATED`: the bits are disjoint, so `|` and `^` give
/// the same value and that mutant would be equivalent (D-0192).
const KNOWN_FLAGS: u8 = 0b11;
const _: () = assert!(KNOWN_FLAGS == FLAG_COMMIT + FLAG_TRUNCATED);

/// Bytes of the store-path digest the header binds itself to.
const PATH_DIGEST_LEN: usize = 24;

/// What kind of thing a landing's bars were read from. D-3334 fixes the
/// three names; the codes are this format's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// A member of a vendor archive on disk (a GDFL day zip's CSV). Code `1`.
    ArchiveMember,
    /// One day's file of the GDFL tick store. Code `2`.
    TickstoreDay,
    /// A vendor's HTTP endpoint (a Zerodha historical-candle request). Code `3`.
    VendorEndpoint,
}

impl SourceKind {
    /// Every kind, in code order.
    pub const ALL: [Self; 3] = [
        Self::ArchiveMember,
        Self::TickstoreDay,
        Self::VendorEndpoint,
    ];

    /// The byte this kind is written as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::ArchiveMember => 1,
            Self::TickstoreDay => 2,
            Self::VendorEndpoint => 3,
        }
    }

    /// The name D-3334 gives this kind, as `/provenance.json` serves it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArchiveMember => "archive_member",
            Self::TickstoreDay => "tickstore_day",
            Self::VendorEndpoint => "vendor_endpoint",
        }
    }

    /// The kind written as `code`, or `None` for any other byte.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }
}

/// A calendar day, written as the `u32` `yyyymmdd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    year: u16,
    month: u8,
    /// The day of the month.
    dom: u8,
}

impl Day {
    /// The day, or `None` when it is not a date of the proleptic Gregorian
    /// calendar between the years 1 and 9999.
    #[must_use]
    pub const fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        if year == 0 || year > 9999 || month == 0 || month > 12 || day == 0 {
            return None;
        }
        let leap =
            (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
        let last = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if day > last {
            return None;
        }
        Some(Self {
            year,
            month,
            dom: day,
        })
    }

    /// The day `yyyymmdd` names, or `None`.
    #[must_use]
    pub fn from_yyyymmdd(packed: u32) -> Option<Self> {
        let year = u16::try_from(packed / 10_000).ok()?;
        let month = u8::try_from(packed / 100 % 100).ok()?;
        let day = u8::try_from(packed % 100).ok()?;
        Self::new(year, month, day)
    }

    /// The day as `yyyymmdd`.
    #[must_use]
    pub fn yyyymmdd(self) -> u32 {
        u32::from(self.year) * 10_000 + u32::from(self.month) * 100 + u32::from(self.dom)
    }

    /// The year.
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.dom
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.dom)
    }
}

/// What a landing read its bars from, as its writer states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source<'a> {
    /// The kind of source.
    pub kind: SourceKind,
    /// Its name. An archive member is named by the archives that hold it,
    /// outermost first, each followed by `!`, then the member's name as the
    /// innermost archive's directory spells it. A tick-store day is its path
    /// below the tick-store root. An endpoint is its path alone: never a query
    /// string, which [`record`] refuses ([`LandingFault::EndpointQuery`]).
    pub name: &'a str,
    /// Its size in bytes: the member's uncompressed size from the archive's
    /// directory, the tick-store file's length, or the response body's.
    pub bytes: u64,
    /// The member's CRC-32 from the archive's directory; `0` for any other
    /// kind.
    pub crc32: u32,
}

/// One landing, as its writer states it. [`record`] adds what the store
/// knows itself: the vendor, the bars appended and the generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landing<'a> {
    /// What the bars were read from.
    pub source: Source<'a>,
    /// The IST day the journal keys this landing by: the GDFL trading day of
    /// the imported file, or the IST day a pull ran.
    pub day: Day,
    /// The bar definition that built the bars (D-2822); `0` where none applies
    /// (bars a vendor built, not this repository).
    pub bar_definition: u32,
    /// The record the landing wrote in its journal: the line of
    /// `imports/gdfl.journal` or the record of `audit/pull.journal`.
    pub journal_ordinal: u64,
    /// The build commit, exactly forty lower-case hexadecimal digits, or
    /// `None` for a build that carried no stamp — written as such, never
    /// guessed.
    pub commit: Option<&'a str>,
    /// When the record is written, UTC epoch microseconds.
    pub landed_at_micros: i64,
}

/// One record, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Its position in the sidecar, from 0.
    pub ordinal: u64,
    /// The feed whose bars these are.
    pub vendor: Vendor,
    /// The kind of source.
    pub source_kind: SourceKind,
    /// The source's size in bytes.
    pub source_bytes: u64,
    /// The archive directory's CRC-32 for a member, else 0.
    pub source_crc32: u32,
    /// The source's name, or its first bytes when
    /// [`Self::source_name_truncated`].
    pub source_name: String,
    /// Whether [`Self::source_name`] is a prefix of the name.
    pub source_name_truncated: bool,
    /// BLAKE3 of the whole name.
    pub source_name_blake3: [u8; 32],
    /// The IST day the journal keys the landing by.
    pub day: Day,
    /// The first bar the landing appended.
    pub first_index: u64,
    /// How many bars it appended. Never zero.
    pub n_bars: u64,
    /// The bar definition that built them, or 0 where none applies.
    pub bar_definition: u32,
    /// The journal record the landing wrote.
    pub journal_ordinal: u64,
    /// The bar file's commit generation that published the bars.
    pub bar_file_generation: u64,
    /// The build commit's twenty bytes, or `None` for an unstamped build.
    pub commit: Option<[u8; 20]>,
    /// When the record was written, UTC epoch microseconds.
    pub landed_at_micros: i64,
}

impl Record {
    /// One past the last bar this record covers.
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.first_index.saturating_add(self.n_bars)
    }

    /// Whether bar `index` is one this landing appended.
    #[must_use]
    pub const fn holds(&self, index: u64) -> bool {
        self.first_index <= index && index < self.end()
    }

    /// The commit as forty lower-case hexadecimal digits.
    #[must_use]
    pub fn commit_hex(&self) -> Option<String> {
        self.commit.map(|bytes| hex(&bytes))
    }

    /// The source-name digest as sixty-four lower-case hexadecimal digits.
    #[must_use]
    pub fn source_name_blake3_hex(&self) -> String {
        hex(&self.source_name_blake3)
    }

    /// The 256 bytes this record is written as.
    ///
    /// | Offset | Size | Field |
    /// |---|---|---|
    /// | 0 | 8 | `vendor` |
    /// | 8 | 1 | `source_kind` |
    /// | 9 | 1 | `flags` |
    /// | 10 | 2 | `name_len` |
    /// | 12 | 4 | `bar_definition` |
    /// | 16 | 8 | `source_bytes` |
    /// | 24 | 4 | `source_crc32` |
    /// | 28 | 4 | `ist_day` |
    /// | 32 | 32 | `source_name_blake3` |
    /// | 64 | 8 | `first_index` |
    /// | 72 | 8 | `n_bars` |
    /// | 80 | 8 | `journal_ordinal` |
    /// | 88 | 8 | `bar_file_generation` |
    /// | 96 | 8 | `landed_at` |
    /// | 104 | 20 | `commit` |
    /// | 124 | 128 | `source_name` |
    /// | 252 | 4 | `record_crc` |
    #[must_use]
    pub fn image(&self) -> [u8; 256] {
        let mut out = [0u8; 256];
        put(&mut out, 0, &vendor_word(self.vendor));
        put(&mut out, 8, &[self.source_kind.code()]);
        let mut flags = 0u8;
        if self.commit.is_some() {
            flags |= FLAG_COMMIT;
        }
        if self.source_name_truncated {
            flags |= FLAG_TRUNCATED;
        }
        put(&mut out, 9, &[flags]);
        let name = self.source_name.as_bytes();
        let name_len = u16::try_from(name.len().min(NAME_CAP)).unwrap_or(0);
        put(&mut out, 10, &name_len.to_le_bytes());
        put(&mut out, 12, &self.bar_definition.to_le_bytes());
        put(&mut out, 16, &self.source_bytes.to_le_bytes());
        put(&mut out, 24, &self.source_crc32.to_le_bytes());
        put(&mut out, 28, &self.day.yyyymmdd().to_le_bytes());
        put(&mut out, 32, &self.source_name_blake3);
        put(&mut out, 64, &self.first_index.to_le_bytes());
        put(&mut out, 72, &self.n_bars.to_le_bytes());
        put(&mut out, 80, &self.journal_ordinal.to_le_bytes());
        put(&mut out, 88, &self.bar_file_generation.to_le_bytes());
        put(&mut out, 96, &self.landed_at_micros.to_le_bytes());
        put(&mut out, 104, &self.commit.unwrap_or([0u8; 20]));
        put(&mut out, 124, name.get(..NAME_CAP).unwrap_or(name));
        let crc = crc32c_split(
            out.get(..252).unwrap_or_default(),
            &self.ordinal.to_le_bytes(),
        );
        put(&mut out, 252, &crc.to_le_bytes());
        out
    }

    /// The record `bytes` holds at `ordinal`.
    ///
    /// # Errors
    ///
    /// [`Fault::Checksum`] when the checksum, which covers the ordinal, does
    /// not match — a torn write, damage, or a record moved from another
    /// position. [`Fault::Invalid`] when it matches but a field holds a value
    /// no writer produces.
    pub fn decode(bytes: &[u8; 256], ordinal: u64) -> Result<Self, Fault> {
        let crc = crc32c_split(bytes.get(..252).unwrap_or_default(), &ordinal.to_le_bytes());
        if crc.to_le_bytes() != take::<4>(bytes, 252) {
            return Err(Fault::Checksum);
        }
        let vendor = vendor_of(take::<8>(bytes, 0)).ok_or(Fault::Invalid("vendor"))?;
        let [code] = take::<1>(bytes, 8);
        let source_kind = SourceKind::from_code(code).ok_or(Fault::Invalid("source_kind"))?;
        let [flags] = take::<1>(bytes, 9);
        if flags & !KNOWN_FLAGS != 0 {
            return Err(Fault::Invalid("flags"));
        }
        let name_len = usize::from(u16::from_le_bytes(take::<2>(bytes, 10)));
        let field = take::<NAME_CAP>(bytes, 124);
        let (stored, padding) = field
            .split_at_checked(name_len)
            .ok_or(Fault::Invalid("name_len"))?;
        let truncated = flags & FLAG_TRUNCATED != 0;
        // A prefix is cut at most three bytes short of the cap, at the last
        // character boundary that fits (`prefix`).
        let short = truncated && name_len + 3 < NAME_CAP;
        if stored.is_empty() || short || padding.iter().any(|&byte| byte != 0) {
            return Err(Fault::Invalid("name_len"));
        }
        let source_name = std::str::from_utf8(stored)
            .map_err(|_| Fault::Invalid("source_name"))?
            .to_owned();
        let source_name_blake3 = take::<32>(bytes, 32);
        if !truncated && brutex_core::blake3::hash(stored) != source_name_blake3 {
            return Err(Fault::Invalid("source_name_blake3"));
        }
        let day = Day::from_yyyymmdd(u32::from_le_bytes(take::<4>(bytes, 28)))
            .ok_or(Fault::Invalid("ist_day"))?;
        let first_index = u64::from_le_bytes(take::<8>(bytes, 64));
        let n_bars = u64::from_le_bytes(take::<8>(bytes, 72));
        if n_bars == 0 || first_index.checked_add(n_bars).is_none() {
            return Err(Fault::Invalid("n_bars"));
        }
        let landed_at_micros = i64::from_le_bytes(take::<8>(bytes, 96));
        if landed_at_micros <= 0 {
            return Err(Fault::Invalid("landed_at"));
        }
        let commit_bytes = take::<20>(bytes, 104);
        let commit = if flags & FLAG_COMMIT != 0 {
            Some(commit_bytes)
        } else if commit_bytes == [0u8; 20] {
            None
        } else {
            return Err(Fault::Invalid("commit"));
        };
        Ok(Self {
            ordinal,
            vendor,
            source_kind,
            source_bytes: u64::from_le_bytes(take::<8>(bytes, 16)),
            source_crc32: u32::from_le_bytes(take::<4>(bytes, 24)),
            source_name,
            source_name_truncated: truncated,
            source_name_blake3,
            day,
            first_index,
            n_bars,
            bar_definition: u32::from_le_bytes(take::<4>(bytes, 12)),
            journal_ordinal: u64::from_le_bytes(take::<8>(bytes, 80)),
            bar_file_generation: u64::from_le_bytes(take::<8>(bytes, 88)),
            commit,
            landed_at_micros,
        })
    }
}

/// Why a record's bytes are not a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The checksum, which covers the record's ordinal, does not match.
    Checksum,
    /// The checksum matches but this field holds a value no writer produces.
    Invalid(&'static str),
}

/// The header at offset 0: which month's bars these records describe.
///
/// | Offset | Size | Field |
/// |---|---|---|
/// | 0 | 8 | magic `BRUTEXP1` |
/// | 8 | 2 | version `1` |
/// | 10 | 2 | record stride `256` |
/// | 12 | 4 | timeframe seconds |
/// | 16 | 4 | symbol id |
/// | 20 | 2 | year |
/// | 22 | 1 | month |
/// | 23 | 1 | reserved, zero |
/// | 24 | 8 | vendor |
/// | 32 | 24 | store-path digest |
/// | 56 | 4 | reserved, zero |
/// | 60 | 4 | CRC-32C over bytes `0..60` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The feed, as the path's first segment names it.
    pub vendor: Vendor,
    /// Seconds per bar, as the bar header says.
    pub timeframe_secs: u32,
    /// The bar header's symbol id.
    pub symbol_id: u32,
    /// The month's year.
    pub year: u16,
    /// The month.
    pub month: u8,
    /// The first 24 bytes of the BLAKE3 of the bar file's store path,
    /// `bars/<vendor>/…/<yyyy-mm>.bin`.
    pub path_digest: [u8; PATH_DIGEST_LEN],
}

impl Header {
    /// The header a sidecar beside `path` carries, for bars of `symbol_id`.
    #[must_use]
    pub fn of(path: StorePath<'_>, symbol_id: u32) -> Self {
        let bars = path.with_file(FileKind::Bars).to_string();
        let mut path_digest = [0u8; PATH_DIGEST_LEN];
        put(
            &mut path_digest,
            0,
            brutex_core::blake3::hash(bars.as_bytes())
                .get(..PATH_DIGEST_LEN)
                .unwrap_or_default(),
        );
        Self {
            vendor: path.vendor(),
            timeframe_secs: path.timeframe().secs(),
            symbol_id,
            year: path.month().year(),
            month: path.month().month(),
            path_digest,
        }
    }

    /// The 64 bytes this header is written as.
    #[must_use]
    pub fn image(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        put(&mut out, 0, &MAGIC);
        put(&mut out, 8, &VERSION.to_le_bytes());
        put(&mut out, 10, &stride_u16().to_le_bytes());
        put(&mut out, 12, &self.timeframe_secs.to_le_bytes());
        put(&mut out, 16, &self.symbol_id.to_le_bytes());
        put(&mut out, 20, &self.year.to_le_bytes());
        put(&mut out, 22, &[self.month]);
        put(&mut out, 24, &vendor_word(self.vendor));
        put(&mut out, 32, &self.path_digest);
        let crc = crc32c(out.get(..60).unwrap_or_default());
        put(&mut out, 60, &crc.to_le_bytes());
        out
    }

    /// The header `bytes` holds.
    ///
    /// Checked in the order a reader can trust each field: the checksum
    /// first, so a torn or rotted header is called that and not a foreign
    /// one; then the magic and the version, so a future version is refused
    /// by number; then the stride and the reserved bytes.
    ///
    /// # Errors
    ///
    /// The [`HeaderFault`] naming the first field that fails.
    pub fn decode(bytes: &[u8; 64]) -> Result<Self, HeaderFault> {
        if crc32c(bytes.get(..60).unwrap_or_default()).to_le_bytes() != take::<4>(bytes, 60) {
            return Err(HeaderFault::Checksum);
        }
        if take::<8>(bytes, 0) != MAGIC {
            return Err(HeaderFault::Magic);
        }
        let version = u16::from_le_bytes(take::<2>(bytes, 8));
        if version != VERSION {
            return Err(HeaderFault::Version(version));
        }
        let stride = u16::from_le_bytes(take::<2>(bytes, 10));
        if stride != stride_u16() {
            return Err(HeaderFault::Stride(stride));
        }
        if take::<1>(bytes, 23) != [0] || take::<4>(bytes, 56) != [0; 4] {
            return Err(HeaderFault::Reserved);
        }
        let vendor = vendor_of(take::<8>(bytes, 24)).ok_or(HeaderFault::Vendor)?;
        Ok(Self {
            vendor,
            timeframe_secs: u32::from_le_bytes(take::<4>(bytes, 12)),
            symbol_id: u32::from_le_bytes(take::<4>(bytes, 16)),
            year: u16::from_le_bytes(take::<2>(bytes, 20)),
            month: u8::from_le_bytes(take::<1>(bytes, 22)),
            path_digest: take::<PATH_DIGEST_LEN>(bytes, 32),
        })
    }

    /// The first field in which this header is not the one `path` implies.
    /// The symbol id is not asked: a path does not carry it.
    fn differs_from(&self, path: StorePath<'_>) -> Option<&'static str> {
        let expected = Self::of(path, self.symbol_id);
        if self.vendor != expected.vendor {
            Some("vendor")
        } else if self.timeframe_secs != expected.timeframe_secs {
            Some("timeframe")
        } else if self.year != expected.year || self.month != expected.month {
            Some("month")
        } else if self.path_digest != expected.path_digest {
            Some("store path")
        } else {
            None
        }
    }
}

/// Why a header's bytes are not a version-1 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderFault {
    /// The checksum over bytes `0..60` does not match.
    Checksum,
    /// The magic is not `BRUTEXP1`.
    Magic,
    /// A version this build does not read.
    Version(u16),
    /// A record stride that is not 256.
    Stride(u16),
    /// A reserved byte is not zero.
    Reserved,
    /// The vendor word names no vendor.
    Vendor,
    /// A field disagrees with the month's path (or, for a writer, with the
    /// bar file's symbol id).
    Mismatch(&'static str),
}

impl fmt::Display for HeaderFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Checksum => f.write_str("its header fails its checksum"),
            Self::Magic => f.write_str("its magic is not BRUTEXP1"),
            Self::Version(version) => write!(f, "it is version {version}, and this build reads 1"),
            Self::Stride(stride) => write!(f, "its record stride is {stride}, not 256"),
            Self::Reserved => f.write_str("a reserved header byte is not zero"),
            Self::Vendor => f.write_str("its vendor word names no vendor"),
            Self::Mismatch(field) => write!(f, "its {field} is not this month's"),
        }
    }
}

/// A month's sidecar as it stands on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    header: Header,
    records: Vec<Record>,
    tail_bytes: u64,
}

impl Sidecar {
    /// The header.
    #[must_use]
    pub const fn header(&self) -> &Header {
        &self.header
    }

    /// Every valid record, in ordinal order, which is also bar order.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// Bytes past the last valid record: an append that was interrupted, or
    /// a last record that fails its checksum, which this format cannot tell
    /// apart. Never served as a record; the next append writes over them.
    /// A reader shows the count rather than hiding it.
    #[must_use]
    pub const fn tail_bytes(&self) -> u64 {
        self.tail_bytes
    }

    /// The record whose landing appended bar `index`, or `None` when no
    /// landing recorded it — a bar written before the month had a sidecar,
    /// or one whose landing died between its commit and its record. A binary
    /// search over at most [`MAX_RECORDS`] records, which are in bar order.
    #[must_use]
    pub fn covering(&self, index: u64) -> Option<&Record> {
        let at = self.records.partition_point(|record| record.end() <= index);
        self.records.get(at).filter(|record| record.holds(index))
    }

    /// Refuses a sidecar that does not describe `bars`: another symbol's, or
    /// one with a record past the bars `bars` has committed — a bar file
    /// replaced after its landings were recorded.
    ///
    /// # Errors
    ///
    /// [`ProvenanceError::Header`] for a symbol-id mismatch,
    /// [`ProvenanceError::PastTheBars`] for a record past the commit.
    pub fn check_bars(&self, path: &Path, bars: &BarFile) -> Result<(), ProvenanceError> {
        let committed = bars.header();
        if self.header.symbol_id != committed.symbol_id {
            return Err(ProvenanceError::Header {
                path: path.to_path_buf(),
                fault: HeaderFault::Mismatch("symbol id"),
            });
        }
        match self.records.last() {
            Some(last) if last.end() > committed.n_valid => Err(ProvenanceError::PastTheBars {
                path: path.to_path_buf(),
                ordinal: last.ordinal,
                end: last.end(),
                n_valid: committed.n_valid,
            }),
            _ => Ok(()),
        }
    }
}

/// What [`read`] found beside a month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// No `.prv`: the month was written before D-3334, or no landing has
    /// recorded itself since.
    Absent,
    /// A `.prv` whose header never reached the disk: shorter than its header,
    /// or nothing but zeros. Its creator died before its first record, so it
    /// holds none; the next landing writes the header again.
    Unborn {
        /// How long the file is.
        bytes: u64,
    },
    /// A sidecar with a valid header.
    Present(Sidecar),
}

/// What [`record`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorded {
    /// The append wrote nothing (`Appended::AlreadyPresent`), so there is no
    /// landing to record and no file was opened.
    NothingAppended,
    /// This record is now durable in the sidecar.
    Written(Record),
}

/// Why a landing a writer offered cannot become a record. Checked before
/// any file is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LandingFault {
    /// The source name is empty.
    EmptyName,
    /// The source name holds a control character.
    ControlCharacter,
    /// An endpoint name carries a query string, which can carry credentials
    /// and is not part of an endpoint's path.
    EndpointQuery,
    /// The commit is not forty lower-case hexadecimal digits.
    Commit,
    /// The record time is not after the epoch.
    LandedAt,
}

impl fmt::Display for LandingFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyName => "the source name is empty",
            Self::ControlCharacter => "the source name holds a control character",
            Self::EndpointQuery => {
                "the endpoint name carries a query string; a record keeps the path alone"
            }
            Self::Commit => "the commit is not forty lower-case hexadecimal digits",
            Self::LandedAt => "the record time is not after the epoch",
        })
    }
}

/// Every refusal this module makes, each naming the file it is about.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProvenanceError {
    /// The path names a sibling other than the bars.
    NotABarPath {
        /// The sibling it names.
        found: FileKind,
    },
    /// The handle is not the bar file the path names.
    WrongBarFile {
        /// The bar file the path names.
        expected: PathBuf,
        /// The one the handle holds.
        handle: PathBuf,
    },
    /// The append is not the handle's newest commit, so the generation that
    /// published it is not the handle's.
    NotTheNewestCommit {
        /// The commit counter the append reported.
        n_valid: u64,
        /// The handle's commit counter now.
        records: u64,
    },
    /// An `Appended::Committed` whose range is empty or reversed.
    ImpossibleRange {
        /// The first index it reported.
        first_index: u64,
        /// The commit counter it reported.
        n_valid: u64,
    },
    /// The landing cannot be written as a record.
    Landing(LandingFault),
    /// A final symlink where the sidecar belongs.
    Symlinked {
        /// The link.
        path: PathBuf,
    },
    /// Something that is not a regular file where the sidecar belongs.
    NotAFile {
        /// What is there.
        path: PathBuf,
    },
    /// Another writer holds the sidecar's lock.
    Locked {
        /// The sidecar.
        path: PathBuf,
    },
    /// The sidecar is longer than any sidecar this format writes.
    TooLarge {
        /// The sidecar.
        path: PathBuf,
        /// Its length.
        bytes: u64,
    },
    /// The header is damaged, foreign, or not this month's.
    Header {
        /// The sidecar.
        path: PathBuf,
        /// What is wrong with it.
        fault: HeaderFault,
    },
    /// A record fails its checksum while a later one passes: damage inside
    /// the history, not an interrupted append.
    Damaged {
        /// The sidecar.
        path: PathBuf,
        /// The damaged record.
        ordinal: u64,
    },
    /// A record passes its checksum and holds an impossible value.
    Record {
        /// The sidecar.
        path: PathBuf,
        /// The record.
        ordinal: u64,
        /// The field.
        field: &'static str,
    },
    /// A record's bars start before the previous record's end, or its
    /// generation is not after the previous record's: the sidecar could not
    /// say which landing wrote a bar.
    OutOfOrder {
        /// The sidecar.
        path: PathBuf,
        /// The record, or the ordinal a refused landing would have taken.
        ordinal: u64,
        /// The previous record's end.
        previous_end: u64,
        /// The previous record's generation.
        previous_generation: u64,
        /// This one's first bar.
        first_index: u64,
        /// This one's generation.
        generation: u64,
    },
    /// The sidecar holds [`MAX_RECORDS`] records.
    Full {
        /// The sidecar.
        path: PathBuf,
    },
    /// A record covers bars the bar file has not committed.
    PastTheBars {
        /// The sidecar.
        path: PathBuf,
        /// The record.
        ordinal: u64,
        /// One past its last bar.
        end: u64,
        /// The bar file's commit counter.
        n_valid: u64,
    },
    /// The host refused an operation.
    Io {
        /// The file or directory.
        path: PathBuf,
        /// What was being done.
        action: &'static str,
        /// The host's error.
        source: io::Error,
    },
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABarPath { found } => write!(
                f,
                "provenance is recorded against the bars, and the path names the {} sibling",
                found.extension()
            ),
            Self::WrongBarFile { expected, handle } => write!(
                f,
                "the path names {} and the handle holds {}",
                expected.display(),
                handle.display()
            ),
            Self::NotTheNewestCommit { n_valid, records } => write!(
                f,
                "the landing committed through bar {n_valid} and the bar file has since committed \
                 through {records}; record each landing before the next append"
            ),
            Self::ImpossibleRange {
                first_index,
                n_valid,
            } => write!(
                f,
                "an append that committed from bar {first_index} to {n_valid} appended nothing"
            ),
            Self::Landing(fault) => write!(f, "the landing cannot be recorded: {fault}"),
            Self::Symlinked { path } => write!(f, "{} is a symlink", path.display()),
            Self::NotAFile { path } => write!(f, "{} is not a regular file", path.display()),
            Self::Locked { path } => write!(f, "{}: another writer holds it", path.display()),
            Self::TooLarge { path, bytes } => write!(
                f,
                "{} is {bytes} bytes, past the {MAX_FILE_LEN} a sidecar of {MAX_RECORDS} \
                 records can be",
                path.display()
            ),
            Self::Header { path, fault } => write!(f, "{}: {fault}", path.display()),
            Self::Damaged { path, ordinal } => write!(
                f,
                "{}: record {ordinal} fails its checksum and a later record passes, so it was \
                 damaged after it was written",
                path.display()
            ),
            Self::Record {
                path,
                ordinal,
                field,
            } => write!(
                f,
                "{}: record {ordinal} passes its checksum and its {field} is impossible",
                path.display()
            ),
            Self::OutOfOrder {
                path,
                ordinal,
                previous_end,
                previous_generation,
                first_index,
                generation,
            } => write!(
                f,
                "{}: record {ordinal} starts at bar {first_index} at generation {generation}, \
                 and the record before it ends at bar {previous_end} at generation \
                 {previous_generation}",
                path.display()
            ),
            Self::Full { path } => write!(
                f,
                "{} holds {MAX_RECORDS} records, the most a sidecar holds",
                path.display()
            ),
            Self::PastTheBars {
                path,
                ordinal,
                end,
                n_valid,
            } => write!(
                f,
                "{}: record {ordinal} covers bars up to {end} and the bar file has committed \
                 {n_valid}",
                path.display()
            ),
            Self::Io {
                path,
                action,
                source,
            } => write!(f, "{}: {action}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ProvenanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Which month the `.prv` beside a bar file says it was written for (DB-B65,
/// WP-81).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrittenFor {
    /// Nothing says: no `.prv`, one never born, one that is not a regular
    /// file, or one whose header fails its own checksum, magic or version.
    /// The month's only on-disk identity is then its bar header's symbol
    /// hash, bar length and stamps.
    Unrecorded,
    /// A whole, sealed header that names this month's own store path.
    ThisMonth,
    /// A whole, sealed header that names a DIFFERENT month: the files were
    /// copied or restored under another vendor, instrument, contract, bar
    /// length or month.
    Another {
        /// The sidecar that says so.
        sidecar: PathBuf,
        /// The first field that is not this month's: `vendor`, `timeframe`,
        /// `month` or `store path` (the instrument or the contract).
        field: &'static str,
    },
}

/// Which month the `.prv` beside the bar file `path` names, under `root`
/// EXACTLY, says it was written for (DB-B65, WP-81).
///
/// # The gap this closes
///
/// A bar header's identity is the low 32 bits of the FNV-1a of the SYMBOL
/// text, so the NIFTY index, every NIFTY option contract and both vendors
/// carry one id (433894009): a month file copied by hand to a sibling strike,
/// or between the index and a contract, opened cleanly and was served as the
/// other instrument. The `.prv` header (D-3334) binds itself to the BLAKE3 of
/// the whole store path — vendor, exchange, segment, symbol, contract, bar
/// length, month — and [`read`] already refuses one that is not its month's;
/// but nothing asked it when the BARS were opened. Every bar door now asks
/// this first.
///
/// # What it refuses, and what it cannot
///
/// [`WrittenFor::Another`] needs positive evidence: a header that passes its
/// own CRC-32C, magic and version and still names another month. A damaged,
/// unborn, foreign-versioned or absent sidecar is [`WrittenFor::Unrecorded`]
/// and refuses nothing here — [`read`] names that damage to whoever asks for
/// the provenance. So a month written before D-3334, which has no `.prv`, is
/// bound by its symbol hash alone until a landing gives it one; that limit is
/// stated, not hidden.
///
/// # Cost
///
/// One `open` (never through a final symlink, never blocking) and one
/// positional read of 64 bytes; for a month with no sidecar, the one `open`
/// that answers `ENOENT`. `root` is used as given: a caller serving a repair
/// revision passes that revision's root.
#[must_use]
pub fn written_for(root: &Path, path: StorePath<'_>) -> WrittenFor {
    let at = path.with_file(FileKind::Provenance).to_path_buf(root);
    let Ok(file) = crate::open_flags::open_read_no_follow(&at) else {
        return WrittenFor::Unrecorded;
    };
    let mut head = [0u8; 64];
    if !file.metadata().is_ok_and(|meta| meta.is_file())
        || file.read_exact_at(&mut head, 0).is_err()
    {
        return WrittenFor::Unrecorded;
    }
    let Ok(header) = Header::decode(&head) else {
        return WrittenFor::Unrecorded;
    };
    match header.differs_from(path) {
        None => WrittenFor::ThisMonth,
        Some(field) => WrittenFor::Another { sidecar: at, field },
    }
}

/// The sidecar beside the month `path` names, if there is one.
///
/// `path` may name any sibling of the month; the sidecar is its `.prv`, beside
/// the generation of the month every reader opens (`crate::generation`): a
/// repaired month's revision has its own, or none. One
/// `open` (never through a final symlink), one `fstat` and one positional read
/// of at most [`MAX_FILE_LEN`] bytes. Takes no lock: records are self-checked
/// and append-only, so a landing being written as this reads shows up as tail
/// bytes, never as a record.
///
/// # Errors
///
/// A [`ProvenanceError`] naming the file and the refusal: a symlink, not a
/// regular file, too long, a damaged or foreign header, a damaged record with
/// a valid one after it, a record with an impossible field, or records out of
/// bar order.
pub fn read(root: &Path, path: StorePath<'_>) -> Result<Found, ProvenanceError> {
    // THE GENERATION EVERY READER OPENS (D-3350, WP-13c). A repaired month's
    // bars are its revision's, and the original's records index the
    // original's bars: read beside the bars that are served, or not at all.
    let served =
        crate::generation::served(root, path.with_file(FileKind::Bars)).map_err(|why| {
            ProvenanceError::Io {
                path: crate::generation::pointer_path(root, path),
                action: "resolving the month's served generation",
                source: io::Error::other(why.to_string()),
            }
        })?;
    let root = &served.root(root);
    // A SEALED MONTH CARRIES ITS PROVENANCE (SC-3, design §4.2.4 and §4.4):
    // the `.prv` image byte for byte, packed inside the `.sbar`. The door
    // asks the sealed form first, as the bar door does.
    if let Some(found) = sealed(root, path)? {
        return Ok(found);
    }
    let at = path.with_file(FileKind::Provenance).to_path_buf(root);
    let file = match crate::open_flags::open_read_no_follow(&at) {
        Ok(file) => file,
        // A RETIREMENT MAY HAVE RUN BETWEEN THE TWO OPENS: the sealed form
        // once more, then absent.
        Err(why) if why.kind() == ErrorKind::NotFound => {
            return Ok(sealed(root, path)?.unwrap_or(Found::Absent));
        }
        Err(why) => return Err(opened(&at, why)),
    };
    let bytes = whole(&file, &at)?;
    match scan(&bytes, path, &at)? {
        Scan::Unborn => Ok(Found::Unborn {
            bytes: len_u64(bytes.len()),
        }),
        Scan::Born(sidecar) => Ok(Found::Present(sidecar)),
    }
}

/// The provenance of `path`'s month from its SEALED form under `root` (the
/// served generation's root), or `None` when there is no `.sbar` there.
///
/// One `open`, two 512-byte head reads (the first takes the head's own symbol
/// id; the path digest the second checks binds the head to the whole store
/// path, which is the stronger check, `refuse_misfiled`'s), and one read of
/// the packed section. A sealed month sealed without a `.prv` is
/// [`Found::Absent`], exactly as its raw form was.
///
/// # Errors
///
/// The open's refusal, and the sealed month's own refusal (head, path,
/// section CRC or length) as [`ProvenanceError::Io`] naming the `.sbar`;
/// then whatever [`scan`] refuses of the unpacked image.
fn sealed(root: &Path, path: StorePath<'_>) -> Result<Option<Found>, ProvenanceError> {
    use crate::sealed::{HEAD_LEN, Head, SealedMonth};
    const _: () = assert!(HEAD_LEN == 512);
    let at = path.with_file(FileKind::SealedBars).to_path_buf(root);
    let file = match crate::open_flags::open_read_no_follow(&at) {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(opened(&at, why)),
    };
    let refused = |why: crate::sealed::SealedError| {
        io_error(
            &at,
            "reading the sealed month's provenance",
            io::Error::other(why.to_string()),
        )
    };
    let mut head = [0u8; 512];
    file.read_exact_at(&mut head, 0)
        .map_err(|why| io_error(&at, "reading the sealed month's head", why))?;
    let symbol_id = Head::decode(&head).map_err(refused)?.symbol_id;
    let month =
        SealedMonth::open(file, path.with_file(FileKind::Bars), symbol_id).map_err(refused)?;
    let Some(image) = month.provenance().map_err(refused)? else {
        return Ok(Some(Found::Absent));
    };
    Ok(Some(match scan(&image, path, &at)? {
        Scan::Unborn => Found::Unborn {
            bytes: len_u64(image.len()),
        },
        Scan::Born(sidecar) => Found::Present(sidecar),
    }))
}

/// Records one landing beside the month it appended to, after its bars are
/// committed.
///
/// `bars` is the writer's open handle and `appended` what its last
/// [`BarFile::append`] returned. `Appended::AlreadyPresent` wrote nothing and
/// records nothing. A committed append becomes record `n` of the sidecar,
/// covering bars `first_index..n_valid` at the handle's generation, with the
/// vendor taken from the path.
///
/// The sidecar is created on the first landing: its header is written and
/// made durable, and its directory synced, before any record. Each record is
/// one positional write followed by `fdatasync`, under an exclusive advisory
/// lock on the `.prv` itself (never the month's `.lock`, which the bar writer
/// holds). The bar file and every other sibling are not opened.
///
/// # Errors
///
/// [`ProvenanceError::NotABarPath`], [`ProvenanceError::WrongBarFile`],
/// [`ProvenanceError::NotTheNewestCommit`], [`ProvenanceError::ImpossibleRange`]
/// or [`ProvenanceError::Landing`] before anything is opened; then anything
/// [`read`] refuses, a header written for another symbol, a landing that
/// overlaps or does not follow the last record
/// ([`ProvenanceError::OutOfOrder`]), a full sidecar, another writer holding
/// the lock, or the host's refusal.
pub fn record(
    root: &Path,
    bars: &BarFile,
    path: StorePath<'_>,
    appended: Appended,
    landing: &Landing<'_>,
) -> Result<Recorded, ProvenanceError> {
    if path.file() != FileKind::Bars {
        return Err(ProvenanceError::NotABarPath { found: path.file() });
    }
    let expected = path.to_path_buf(root);
    if bars.path() != expected {
        return Err(ProvenanceError::WrongBarFile {
            expected,
            handle: bars.path().to_path_buf(),
        });
    }
    let (first_index, n_valid) = match appended {
        Appended::AlreadyPresent { .. } => return Ok(Recorded::NothingAppended),
        Appended::Committed {
            first_index,
            n_valid,
        } => (first_index, n_valid),
    };
    let committed = bars.header();
    if n_valid != committed.n_valid {
        return Err(ProvenanceError::NotTheNewestCommit {
            n_valid,
            records: committed.n_valid,
        });
    }
    let n_bars = n_valid.checked_sub(first_index).filter(|&n| n > 0).ok_or(
        ProvenanceError::ImpossibleRange {
            first_index,
            n_valid,
        },
    )?;
    let mut fresh = landed(
        path.vendor(),
        landing,
        first_index,
        n_bars,
        committed.generation,
    )
    .map_err(ProvenanceError::Landing)?;

    let at = path.with_file(FileKind::Provenance).to_path_buf(root);
    refuse_odd(&at)?;
    // Inside a durable batch (WP-16, D-3353) the record is written as every
    // other write of the batch is, and its flushes are the batch's.
    let batch = crate::file::DurableBatch::current();
    let file = opened_rw(&at, batch.as_ref())?;
    let file = Flock::try_lock(file, at.clone()).map_err(|why| match why {
        fs::TryLockError::WouldBlock => ProvenanceError::Locked { path: at.clone() },
        fs::TryLockError::Error(why) => io_error(&at, "lock", why),
    })?;
    let bytes = whole(&file, &at)?;
    let sidecar = match scan(&bytes, path, &at)? {
        Scan::Born(sidecar) => sidecar,
        Scan::Unborn => {
            write_fully(
                &file,
                &at,
                0,
                &Header::of(path, committed.symbol_id).image(),
            )?;
            sync_data(&file, &at, batch.as_ref())?;
            sync_parent(&at, batch.as_ref())?;
            Sidecar {
                header: Header::of(path, committed.symbol_id),
                records: Vec::new(),
                tail_bytes: 0,
            }
        }
    };
    if sidecar.header.symbol_id != committed.symbol_id {
        return Err(ProvenanceError::Header {
            path: at,
            fault: HeaderFault::Mismatch("symbol id"),
        });
    }
    let ordinal = len_u64(sidecar.records.len());
    if ordinal >= MAX_RECORDS {
        return Err(ProvenanceError::Full { path: at });
    }
    fresh.ordinal = ordinal;
    if let Some(previous) = sidecar.records.last() {
        follows(previous, &fresh, &at)?;
    }
    write_fully(&file, &at, offset_of(ordinal), &fresh.image())?;
    sync_data(&file, &at, batch.as_ref())?;
    Ok(Recorded::Written(fresh))
}

/// What a file's bytes are: unborn, or a sidecar.
enum Scan {
    Unborn,
    Born(Sidecar),
}

/// Decodes a whole sidecar: the header against the month `path` names, then
/// the longest prefix of valid records, then the tail.
fn scan(bytes: &[u8], path: StorePath<'_>, at: &Path) -> Result<Scan, ProvenanceError> {
    let Some(head) = bytes.first_chunk::<64>() else {
        return Ok(Scan::Unborn);
    };
    if bytes.iter().all(|&byte| byte == 0) {
        return Ok(Scan::Unborn);
    }
    let header = Header::decode(head).map_err(|fault| ProvenanceError::Header {
        path: at.to_path_buf(),
        fault,
    })?;
    if let Some(field) = header.differs_from(path) {
        return Err(ProvenanceError::Header {
            path: at.to_path_buf(),
            fault: HeaderFault::Mismatch(field),
        });
    }
    let body = bytes.get(64..).unwrap_or_default();
    let (chunks, _) = body.as_chunks::<256>();
    let mut records: Vec<Record> = Vec::with_capacity(chunks.len());
    // THE FIRST RECORD THAT FAILS ITS CHECKSUM ENDS THE HISTORY, unless a
    // later one passes: a write only ever lands at the end, so a sealed record
    // after an unsealed one means the unsealed one was damaged in place.
    let mut first_bad = None;
    for (ordinal, chunk) in (0u64..).zip(chunks) {
        let decoded = Record::decode(chunk, ordinal);
        let sealed = !matches!(decoded, Err(Fault::Checksum));
        if let Some(bad) = first_bad {
            if sealed {
                return Err(ProvenanceError::Damaged {
                    path: at.to_path_buf(),
                    ordinal: bad,
                });
            }
            continue;
        }
        let found = match decoded {
            Ok(found) => found,
            Err(Fault::Checksum) => {
                first_bad = Some(ordinal);
                continue;
            }
            Err(Fault::Invalid(field)) => {
                return Err(ProvenanceError::Record {
                    path: at.to_path_buf(),
                    ordinal,
                    field,
                });
            }
        };
        if found.vendor != header.vendor {
            return Err(ProvenanceError::Record {
                path: at.to_path_buf(),
                ordinal,
                field: "vendor",
            });
        }
        if let Some(previous) = records.last() {
            follows(previous, &found, at)?;
        }
        records.push(found);
    }
    let tail_bytes = len_u64(bytes.len()).saturating_sub(offset_of(len_u64(records.len())));
    Ok(Scan::Born(Sidecar {
        header,
        records,
        tail_bytes,
    }))
}

/// Refuses `next` unless its bars start at or after `previous`'s end and its
/// generation is after `previous`'s.
fn follows(previous: &Record, next: &Record, at: &Path) -> Result<(), ProvenanceError> {
    if next.first_index >= previous.end() && next.bar_file_generation > previous.bar_file_generation
    {
        return Ok(());
    }
    Err(ProvenanceError::OutOfOrder {
        path: at.to_path_buf(),
        ordinal: next.ordinal,
        previous_end: previous.end(),
        previous_generation: previous.bar_file_generation,
        first_index: next.first_index,
        generation: next.bar_file_generation,
    })
}

/// The record a landing becomes, its ordinal still 0.
fn landed(
    vendor: Vendor,
    landing: &Landing<'_>,
    first_index: u64,
    n_bars: u64,
    generation: u64,
) -> Result<Record, LandingFault> {
    let name = landing.source.name;
    if name.is_empty() {
        return Err(LandingFault::EmptyName);
    }
    if name.chars().any(char::is_control) {
        return Err(LandingFault::ControlCharacter);
    }
    if landing.source.kind == SourceKind::VendorEndpoint && name.contains('?') {
        return Err(LandingFault::EndpointQuery);
    }
    if landing.landed_at_micros <= 0 {
        return Err(LandingFault::LandedAt);
    }
    let commit = landing.commit.map(commit_bytes).transpose()?;
    let kept = prefix(name, NAME_CAP);
    Ok(Record {
        ordinal: 0,
        vendor,
        source_kind: landing.source.kind,
        source_bytes: landing.source.bytes,
        source_crc32: landing.source.crc32,
        source_name: kept.to_owned(),
        source_name_truncated: kept.len() < name.len(),
        source_name_blake3: brutex_core::blake3::hash(name.as_bytes()),
        day: landing.day,
        first_index,
        n_bars,
        bar_definition: landing.bar_definition,
        journal_ordinal: landing.journal_ordinal,
        bar_file_generation: generation,
        commit,
        landed_at_micros: landing.landed_at_micros,
    })
}

/// The longest prefix of `name` that fits `cap` bytes and ends on a
/// character boundary.
fn prefix(name: &str, cap: usize) -> &str {
    name.get(..name.floor_char_boundary(cap))
        .unwrap_or_default()
}

/// A forty-digit lower-case commit as its twenty bytes.
fn commit_bytes(text: &str) -> Result<[u8; 20], LandingFault> {
    let digits = text.as_bytes();
    if digits.len() != 40 {
        return Err(LandingFault::Commit);
    }
    let mut out = [0u8; 20];
    for (byte, pair) in out.iter_mut().zip(digits.chunks_exact(2)) {
        let [high, low] = [pair.first(), pair.get(1)]
            .map(|digit| digit.copied().and_then(nibble).ok_or(LandingFault::Commit));
        *byte = high? * 16 + low?;
    }
    Ok(out)
}

/// The value of one lower-case hexadecimal digit.
const fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

/// Bytes as lower-case hexadecimal.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        for nibble in [byte >> 4, byte & 0xF] {
            out.push(char::from(
                DIGITS.get(usize::from(nibble)).copied().unwrap_or(b'0'),
            ));
        }
    }
    out
}

/// The eight bytes a vendor is written as: its name, zero-padded.
fn vendor_word(vendor: Vendor) -> [u8; 8] {
    let mut out = [0u8; 8];
    put(&mut out, 0, vendor.as_str().as_bytes());
    out
}

/// The vendor whose word `word` is, or `None`.
fn vendor_of(word: [u8; 8]) -> Option<Vendor> {
    Vendor::ALL
        .into_iter()
        .find(|&vendor| vendor_word(vendor) == word)
}

/// Where record `ordinal` starts.
const fn offset_of(ordinal: u64) -> u64 {
    HEADER_LEN.saturating_add(ordinal.saturating_mul(RECORD_STRIDE))
}

/// [`RECORD_STRIDE`] as the header's `u16`.
const STRIDE_U16: u16 = 256;
const _: () = assert!(STRIDE_U16 as u64 == RECORD_STRIDE);

/// The stride as the header writes it.
const fn stride_u16() -> u16 {
    STRIDE_U16
}

/// `usize` to `u64`; lossless on every target this crate builds for.
fn len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

/// The whole file, read once, after its length is checked against
/// [`MAX_FILE_LEN`].
fn whole(file: &File, at: &Path) -> Result<Vec<u8>, ProvenanceError> {
    let meta = file.metadata().map_err(|why| io_error(at, "stat", why))?;
    if !meta.is_file() {
        return Err(ProvenanceError::NotAFile {
            path: at.to_path_buf(),
        });
    }
    if meta.len() > MAX_FILE_LEN {
        return Err(ProvenanceError::TooLarge {
            path: at.to_path_buf(),
            bytes: meta.len(),
        });
    }
    let mut out = vec![0u8; usize::try_from(meta.len()).unwrap_or(0)];
    // A file that shrank after its `fstat` — nothing in this format truncates
    // one — is a named refusal (`UnexpectedEof`), never a shorter read.
    file.read_exact_at(&mut out, 0)
        .map_err(|why| io_error(at, "read", why))?;
    Ok(out)
}

/// Writes every byte at `offset`. A partial write is resumed and a write
/// that accepts nothing is refused (`WriteZero`) by `write_all_at`; what did
/// reach the file is tail, which the next landing writes over.
fn write_fully(file: &File, at: &Path, offset: u64, bytes: &[u8]) -> Result<(), ProvenanceError> {
    file.write_all_at(bytes, offset)
        .map_err(|why| io_error(at, "write", why))
}

/// Makes what was written to the sidecar durable, or leaves it to `batch`
/// while the batch is open (WP-16, D-3353).
fn sync_data(
    file: &File,
    at: &Path,
    batch: Option<&crate::file::DurableBatch>,
) -> Result<(), ProvenanceError> {
    if let Some(batch) = batch {
        let deferred = batch
            .defer_file(file, at)
            .map_err(|why| ProvenanceError::Io {
                path: at.to_path_buf(),
                action: "measure the device of",
                source: io::Error::other(why.to_string()),
            })?;
        if deferred {
            return Ok(());
        }
    }
    file.sync_data().map_err(|why| io_error(at, "sync", why))
}

/// Makes a new directory entry durable, or leaves it to `batch` while the
/// batch is open (WP-16, D-3353).
fn sync_parent(
    at: &Path,
    batch: Option<&crate::file::DurableBatch>,
) -> Result<(), ProvenanceError> {
    let dir = at.parent().unwrap_or(at);
    if batch.is_some_and(|batch| batch.defer_directory(dir)) {
        return Ok(());
    }
    File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| io_error(dir, "sync directory", why))
}

/// [`ProvenanceError::Symlinked`] when `at` itself is a link, and
/// [`ProvenanceError::NotAFile`] when it is anything else but a regular
/// file. Asked with `symlink_metadata`, which never follows; an absent path
/// is neither.
fn refuse_odd(at: &Path) -> Result<(), ProvenanceError> {
    match fs::symlink_metadata(at) {
        Ok(meta) if meta.file_type().is_symlink() => Err(ProvenanceError::Symlinked {
            path: at.to_path_buf(),
        }),
        Ok(meta) if !meta.is_file() => Err(ProvenanceError::NotAFile {
            path: at.to_path_buf(),
        }),
        _ => Ok(()),
    }
}

/// Opens for reading and writing, creating but never truncating, and never
/// through a final symlink on the hosts `crate::open_flags` has a value for.
fn opened_rw(
    at: &Path,
    batch: Option<&crate::file::DurableBatch>,
) -> Result<File, ProvenanceError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = File::options();
    options.read(true).write(true).create(true).truncate(false);
    // `O_NOFOLLOW` where it is verified, as the bar writer opens; and
    // `O_SYNC` too inside a batch that writes through (WP-16, D-3353).
    options.custom_flags(crate::file::writer_flags(
        batch.is_some_and(crate::file::DurableBatch::writes_through),
    ));
    options.open(at).map_err(|why| opened(at, why))
}

/// An open's failure, named: a final symlink refused under `O_NOFOLLOW`
/// (`ELOOP`) is [`ProvenanceError::Symlinked`].
fn opened(at: &Path, why: io::Error) -> ProvenanceError {
    #[cfg(any(
        target_os = "macos",
        all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
        return ProvenanceError::Symlinked {
            path: at.to_path_buf(),
        };
    }
    io_error(at, "open", why)
}

/// [`ProvenanceError::Io`].
fn io_error(at: &Path, action: &'static str, source: io::Error) -> ProvenanceError {
    ProvenanceError::Io {
        path: at.to_path_buf(),
        action,
        source,
    }
}

/// Writes `bytes` into `out` at `at`, a no-op past its end. Every call site
/// passes a fixed offset inside a fixed array.
fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
        dst.copy_from_slice(bytes);
    }
}

/// `N` bytes of `bytes` from `at`, or zeros past its end — see [`put`].
fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(run) = bytes.get(at..at.saturating_add(N)) {
        out.copy_from_slice(run);
    }
    out
}
