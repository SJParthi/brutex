//! The verdict store: what verification found, per (feed, instrument, day,
//! timeframe), in immutable day images under a two-slot year index.
//!
//! `docs/02-store-format.md` §35 is the authority for every byte here. The
//! decisions are D-4808 (the store), D-4809 (append-only numbering), D-4844
//! (format version 1: its codes, flags, the no-record answer, the record
//! ceiling and the month answers a staleness probe hands a lookup), D-4865
//! (images are linked from a writer-unique temporary, never renamed over),
//! D-4879 (code 18, `owed`) and D-4861 (the ledger rows land with the code).
//! The staleness rule itself, and the month probe that computes it, are
//! IA-6's (its decision lands with it); this module only compares what the
//! probe read with what a record holds. Package IA-5 of the
//! ingest-automation design builds it; the verifiers that write it
//! (`pull::gdfl_verify`, `pull::zerodha_verify`) and the routes that read it
//! are later packages.
//!
//! # Layout
//!
//! ```text
//! <store>/verify/<feed>/<EXCHANGE>/<SEGMENT>/<tf>/<yyyy>.vdi                            year index
//! <store>/verify/<feed>/<EXCHANGE>/<SEGMENT>/<tf>/<yyyy>.vdi.lock                       commit lock
//! <store>/verify/<feed>/<EXCHANGE>/<SEGMENT>/<tf>/<yyyy>/<yyyy-mm-dd>.g<NNNNNNNNNN>.vdt  day image
//! ```
//!
//! The four directory levels are the bar store's own, in its order
//! (`bars/<vendor>/<exchange>/<segment>/…/<timeframe>/`): two exchanges have
//! two calendars, so one day is two day records, never one shared between
//! them (D-4923). Nothing here lists the feeds, the exchanges or the
//! timeframes: a feed is its position in `Vendor::ALL`, an exchange and a
//! segment are `core`'s enums matched exhaustively, and a timeframe is its
//! own number of seconds, read back through [`Timeframe::from_secs`].
//!
//! The generation in an image's name is zero-padded to 10 digits, the width
//! of `u32::MAX`, so every generation has one name and one day's names sort in
//! generation order.
//!
//! A day image holds one 160-byte [`Record`] per key verified that day,
//! sorted by (code, key), a tagged open-addressing table that finds a key's
//! record, a detail section of variable-length facts, and a BLAKE3 trailer.
//! The year index holds, per day of the year, which image generation is
//! current, the day's worst code and its counts. It commits through two
//! header slots, the bar file's own rule (`docs/02-store-format.md` §2).
//!
//! # What is append-only here
//!
//! Every image ever linked is kept. A re-verification writes generation
//! `g + 1` and commits the year index to point at it; image `g` stays byte
//! for byte (`crates/store/tests/verdict.rs`,
//! `a_reverified_day_is_a_new_generation_and_the_old_is_kept`). Nothing here
//! opens an image for writing once it is linked into place.
//!
//! # Cost
//!
//! [`Reader::lookup`] reads at most 5 times and opens at most 2 files,
//! whatever the day size, the year or the history: 2 slot reads of the year
//! index, 1 read of the image's 512-byte header and day block, 1 read of the
//! 64-byte probe window and 1 read of a 160-byte record. Each answer carries
//! its own [`Cost`], and `crates/store/tests/verdict.rs`
//! (`a_verdict_is_found_by_one_bounded_probe_at_every_day_size`, IA-VD-01)
//! asserts it at 1x, 10x and 50x a day's records; `crates/store/benches/ratio.rs`
//! (C-VD-01) times it. 100x is past the format's record ceiling and is
//! refused `table_too_large` with nothing written (L-IA-33). [`commit`] is
//! not constant: it sorts the day's records once, O(n log n) for n records,
//! and makes 3 `sync_all`s (L-IA-4; C-VD-04).
//!
//! # What `store` does not know
//!
//! The reason a record carries is an opaque `u16` and a detail's kind is an
//! opaque `u16`: their tables live in `pull` (D-4809), which `store` cannot
//! name. The one exception is detail kind [`DETAIL_NAME`], which this module
//! writes itself when it cuts a stem longer than a record's key field.
//! Whether a verdict is stale against the month it describes needs the month
//! (`pull`'s probe, IA-6): [`Answer::staleness`] compares what the caller
//! read ([`MonthNow`]) with what the record holds, and says which checks the
//! caller did not make.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, File, TryLockError};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use brutex_core::blake3;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;

use crate::crc::crc32c;
use crate::flock::Flock;
use crate::path::Timeframe;
use crate::provenance::Day;
use crate::staging::{self, StagingError};

// ------------------------------------------------------------- constants --

/// The magic at byte 0 of every day image.
pub const IMAGE_MAGIC: [u8; 8] = *b"BRUTEXVD";

/// The magic at byte 0 of every year-index slot.
pub const INDEX_MAGIC: [u8; 8] = *b"BRUTEXVI";

/// The day-image version this build reads and writes. A change to any byte
/// of §35 is a new version, never a reinterpretation (`CLAUDE.md` §3 rule 8).
pub const IMAGE_VERSION: u16 = 1;

/// The year-index version this build reads and writes.
pub const INDEX_VERSION: u16 = 1;

/// Bytes of one record.
pub const RECORD_STRIDE: usize = 160;

/// Bytes of the image header at offset 0.
pub const HEADER_LEN: usize = 256;

/// Bytes of the day block at offset [`HEADER_LEN`].
pub const DAY_BLOCK_LEN: usize = 256;

/// Where the hash table starts: after the header and the day block.
pub const TABLE_OFFSET: usize = HEADER_LEN + DAY_BLOCK_LEN;

/// Bytes of one table slot: `{u32 ordinal + 1, u32 tag}`.
pub const SLOT_LEN: usize = 8;

/// Slots past slot `c − 1`, so a probe window never wraps.
pub const OVERFLOW_SLOTS: usize = 7;

/// The probe window: a key's record sits within this many slots of its home.
pub const MAX_PROBE: usize = 8;

/// The smallest table capacity.
pub const MIN_CAPACITY: usize = 16;

/// The largest table capacity; past it a day is refused `table_too_large`.
/// With an 8-slot window and no repeated tag, the table needs about 3n to
/// 10n slots, so this caps the records one image holds: a named, measured
/// ceiling (L-IA-33: 137,900 records placed, 275,800 refused), not a proof.
pub const MAX_CAPACITY: usize = 1 << 20;

/// The most records one made-up key set has been placed at (L-IA-33). A
/// measurement, not a bound the format guarantees: whether a day's keys fit
/// depends on the keys, and of 200 random key sets of this size 184 fit and
/// 16 are refused (`crates/store/tests/verdict.rs`,
/// `a_days_keys_fit_whatever_the_keys_are`).
pub const RECORDS_PLACED: u32 = 137_900;

/// Half of [`RECORDS_PLACED`]: a commit of at least this many records says
/// so ([`Committed::near_ceiling`]), so the day that halves the margin is
/// named while every day still fits: 1,000 of 1,000 random key sets of this
/// size are placed (RA-261; D-4925).
pub const RECORDS_WATCH: u32 = RECORDS_PLACED / 2;

/// Bytes of a record's key field.
pub const KEY_LEN: usize = 64;

/// The largest detail section. Past it the day carries
/// [`flags::DETAILS_CAPPED`] and the remaining records keep their counts only.
pub const DETAIL_CAP: usize = 4 << 20;

/// Bytes of a detail entry's head: `{u32 ordinal + 1, u16 kind, u16 length}`.
pub const DETAIL_HEAD_LEN: usize = 8;

/// Bytes of the BLAKE3 trailer at the end of an image.
pub const TRAILER_LEN: usize = 32;

/// Counts per verdict code kept in the header: codes 0 to 23.
pub const CODE_SLOTS: usize = 24;

/// The one detail kind `store` writes itself: the whole name of a stem whose
/// key field holds only its first [`KEY_LEN`] bytes. `pull`'s `DETAILS` table
/// numbers it 1, `name` (D-4809, §6.4 of the design).
pub const DETAIL_NAME: u16 = 1;

/// A `u32` field's "none".
pub const NONE: u32 = u32::MAX;

/// Where the year index's second slot starts: the bar file's slot stride.
pub const INDEX_SLOT_STRIDE: usize = 16_384;

/// [`INDEX_SLOT_STRIDE`] as a file offset.
const SLOT_STRIDE_U64: u64 = 16_384;

/// Bytes of a year-index slot that are used.
pub const INDEX_SLOT_LEN: usize = 8_812;

/// The whole year index: two slots.
pub const INDEX_LEN: usize = 2 * INDEX_SLOT_STRIDE;

/// [`INDEX_LEN`] as a file length.
const INDEX_LEN_U64: u64 = 32_768;

/// Day entries per year-index slot.
pub const YEAR_DAYS: usize = 366;

/// Bytes of one day entry.
pub const DAY_ENTRY_LEN: usize = 24;

/// Bytes of a year-index slot before its day entries.
const INDEX_HEAD_LEN: usize = 24;

/// How many generations a writer steps past an image a crash left linked
/// and uncommitted before it refuses `generation_collision`.
pub const MAX_LINK_TRIES: u32 = 16;

/// How long [`commit`] waits for the commit lock before it refuses
/// `lock_stalled` (D-4924, adopting the critic's D-4906). A holder that dies
/// frees its lock at once; one that hangs on a stalled drive would otherwise
/// hold every later commit of the year for ever, with no refusal and no
/// event (L-IA-41).
pub const LOCK_WAIT: Duration = Duration::from_secs(30);

/// One step of the counted wait for the commit lock.
pub const LOCK_STEP: Duration = Duration::from_millis(10);

/// The reader's handle cache: year indexes and day images, least recently
/// used out (D-4844).
pub const HANDLE_CACHE: usize = 64;

const _: () = assert!(INDEX_HEAD_LEN + YEAR_DAYS * DAY_ENTRY_LEN + 4 == INDEX_SLOT_LEN);
const _: () = assert!(4 + 1 + 1 + 2 + 4 + 4 + 8 == DAY_ENTRY_LEN);
const _: () = assert!(INDEX_SLOT_LEN <= INDEX_SLOT_STRIDE);
const _: () = assert!(INDEX_LEN_U64 == 2 * SLOT_STRIDE_U64 && SLOT_STRIDE_U64 == 16_384);
const _: () = assert!(32 + CODE_SLOTS * 4 == 128);

// ----------------------------------------------------------------- codes --

/// A verdict code (§6.4 of the design; D-4809: numbered append-only, never
/// renamed or reused). The number is the byte on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Code {
    /// Every unit equal, none extra.
    Match = 1,
    /// An OHLC, volume or OI value differs on a unit both hold.
    ValueMismatch = 2,
    /// The store lacks a unit that a close in scope says was imported, or a
    /// Zerodha minute a covering gap ledger does not explain. A unit no
    /// import has landed yet is [`Code::Owed`], never this (D-4879).
    MissingInStore = 3,
    /// The store holds a unit the oracle did not make from that day's source.
    ExtraInStore = 4,
    /// The journal, census or listing counts disagree with the store.
    CountMismatch = 5,
    /// The importer refused the file by name.
    SourceRefused = 6,
    /// The store holds bars for a key the source lacks that day.
    StoreWithoutSource = 7,
    /// The file read clean, filed nothing, and was not refused.
    AbsentFromStore = 8,
    /// The month file refused to open, or the read-back met a stamp that does
    /// not increase.
    StoreUnreadable = 9,
    /// Zerodha: no kept answer for these minutes.
    NoResponseKept = 10,
    /// Policy: not compared.
    NotCompared = 11,
    /// A calendar-open day the source does not hold.
    SourceLacksDay = 12,
    /// Owed minutes the vendor answered empty.
    VendorHole = 13,
    /// An owed unit withheld by a recorded rule.
    WithheldByRule = 14,
    /// Built under a rule set this verifier does not compare by value.
    BuiltUnderEarlierRule = 15,
    /// The day closed partial and this key was not reached.
    DayPartial = 16,
    /// Zerodha owed minutes missing, with no gap ledger to say why.
    NoLedger = 17,
    /// The source holds the unit, today's scope owes it, and no import has
    /// landed it yet: never green and never a store defect (D-4879).
    Owed = 18,
}

impl Code {
    /// Every code version 1 mints, in code order: codes 1 to 18 (D-4844,
    /// D-4879). Code 0 is unset and never written.
    pub const ALL: [Self; 18] = [
        Self::Match,
        Self::ValueMismatch,
        Self::MissingInStore,
        Self::ExtraInStore,
        Self::CountMismatch,
        Self::SourceRefused,
        Self::StoreWithoutSource,
        Self::AbsentFromStore,
        Self::StoreUnreadable,
        Self::NoResponseKept,
        Self::NotCompared,
        Self::SourceLacksDay,
        Self::VendorHole,
        Self::WithheldByRule,
        Self::BuiltUnderEarlierRule,
        Self::DayPartial,
        Self::NoLedger,
        Self::Owed,
    ];

    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The code a byte names, or `None` for 0 (unset) and for a code this
    /// build does not know.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::Match,
            2 => Self::ValueMismatch,
            3 => Self::MissingInStore,
            4 => Self::ExtraInStore,
            5 => Self::CountMismatch,
            6 => Self::SourceRefused,
            7 => Self::StoreWithoutSource,
            8 => Self::AbsentFromStore,
            9 => Self::StoreUnreadable,
            10 => Self::NoResponseKept,
            11 => Self::NotCompared,
            12 => Self::SourceLacksDay,
            13 => Self::VendorHole,
            14 => Self::WithheldByRule,
            15 => Self::BuiltUnderEarlierRule,
            16 => Self::DayPartial,
            17 => Self::NoLedger,
            18 => Self::Owed,
            _ => return None,
        })
    }

    /// The word the routes and the documents use.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::ValueMismatch => "value_mismatch",
            Self::MissingInStore => "missing_in_store",
            Self::ExtraInStore => "extra_in_store",
            Self::CountMismatch => "count_mismatch",
            Self::SourceRefused => "source_refused",
            Self::StoreWithoutSource => "store_without_source",
            Self::AbsentFromStore => "absent_from_store",
            Self::StoreUnreadable => "store_unreadable",
            Self::NoResponseKept => "no_response_kept",
            Self::NotCompared => "not_compared",
            Self::SourceLacksDay => "source_lacks_day",
            Self::VendorHole => "vendor_hole",
            Self::WithheldByRule => "withheld_by_rule",
            Self::BuiltUnderEarlierRule => "built_under_earlier_rule",
            Self::DayPartial => "day_partial",
            Self::NoLedger => "no_ledger",
            Self::Owed => "owed",
        }
    }

    /// The severity rank, 1 the worst (D-4844, with code 18 `owed` placed
    /// at 9 by D-4879). It decides a day's worst code, the year index's
    /// per-day worst and every badge's colour.
    #[must_use]
    pub const fn severity(self) -> u8 {
        match self {
            Self::StoreUnreadable => 1,
            Self::ValueMismatch => 2,
            Self::MissingInStore => 3,
            Self::ExtraInStore => 4,
            Self::StoreWithoutSource => 5,
            Self::AbsentFromStore => 6,
            Self::CountMismatch => 7,
            Self::DayPartial => 8,
            Self::Owed => 9,
            Self::SourceRefused => 10,
            Self::SourceLacksDay => 11,
            Self::VendorHole => 12,
            Self::WithheldByRule => 13,
            Self::NoLedger => 14,
            Self::BuiltUnderEarlierRule => 15,
            Self::NoResponseKept => 16,
            Self::NotCompared => 17,
            Self::Match => 18,
        }
    }

    /// Whether the code is a store defect, a "mismatch": 9, 2, 3, 4, 7, 8, 5.
    #[must_use]
    pub const fn is_mismatch(self) -> bool {
        self.severity() <= 7
    }

    /// The worse of two codes by [`Code::severity`].
    #[must_use]
    pub const fn worse(self, other: Self) -> Self {
        if other.severity() < self.severity() {
            other
        } else {
            self
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// Verdict flags: verification facts only, bits appended and never reused
/// (D-4809, D-4841).
///
/// Format version 1 names bits 0 to 21 and no others (D-4844). **A writer
/// refuses a record carrying any bit it does not name** (`flag_unknown`,
/// nothing written), so no build can write a fact it cannot name. **A reader
/// never refuses or drops a bit it does not name**: an image a later build
/// wrote with bit 22 or above reads whole, and [`flags::words`] names the bit
/// `flag_<n>`, because bits are appended and a newer image must stay readable
/// by an older reader. A later bit is appended to [`flags::ALL`] by the package that
/// first sets it, in the same change.
pub mod flags {
    use std::borrow::Cow;

    /// The month has no `.tix` time index.
    pub const TIX_ABSENT: u64 = 1;
    /// Option seconds without a spot second.
    pub const SPOT_MISSING: u64 = 1 << 1;
    /// Option seconds after the index close.
    pub const AFTER_INDEX_CLOSE: u64 = 1 << 2;
    /// The underlying's day is not stored.
    pub const SPOT_UNAVAILABLE: u64 = 1 << 3;
    /// Zerodha minutes absent while GDFL traded.
    pub const PEER_MINUTE_ABSENT: u64 = 1 << 4;
    /// The peer check could not run.
    pub const PEER_UNAVAILABLE: u64 = 1 << 5;
    /// The vendor's answer was not kept.
    pub const BODY_NOT_KEPT: u64 = 1 << 6;
    /// The source changed since it was imported.
    pub const SOURCE_CHANGED_SINCE_IMPORT: u64 = 1 << 7;
    /// An archive day on a calendar-closed day.
    pub const ARCHIVE_DAY_ON_CLOSED_DAY: u64 = 1 << 8;
    /// The `_v3` manifest disagrees.
    pub const V3_MANIFEST_DISAGREES: u64 = 1 << 9;
    /// A stem hands over to another with no alias row.
    pub const LINEAGE_UNRECORDED: u64 = 1 << 10;
    /// The detail section reached [`super::DETAIL_CAP`]; this record's or the
    /// day's details were not kept.
    pub const DETAILS_CAPPED: u64 = 1 << 11;
    /// The key field holds the first [`super::KEY_LEN`] bytes of the name.
    pub const NAME_CUT: u64 = 1 << 12;
    /// Identical duplicate minutes were counted once.
    pub const DUP_MINUTES: u64 = 1 << 13;
    /// The vendor revised an answer.
    pub const VENDOR_REVISED: u64 = 1 << 14;
    /// A traded row past the venue close on the expiry day.
    pub const EXPIRY_AFTER_CLOSE: u64 = 1 << 15;
    /// The verifier's count of a ledger field differs from the ledger's.
    pub const QUALITY_DIFFERS: u64 = 1 << 16;
    /// A cross-kind input is still pending.
    pub const DEFERRED_PENDING: u64 = 1 << 17;
    /// `daycheck` differs.
    pub const DAYCHECK_DIFFERS: u64 = 1 << 18;
    /// The serve path differed from the stored rows.
    pub const SERVE_PATH_DIFFERS: u64 = 1 << 19;
    /// The lineage hand-over detector has not landed, so a hand-over cannot
    /// be judged (WP-12f's detector; set by IA-6).
    pub const LINEAGE_UNCHECKED: u64 = 1 << 20;
    /// The stored day has no archive-member landing record (`.prv`) to
    /// compare its source CRC with, so [`SOURCE_CHANGED_SINCE_IMPORT`]
    /// cannot be judged (set by IA-6).
    pub const SOURCE_CRC_UNRECORDED: u64 = 1 << 21;

    /// Every bit version 1 names, with its word, in bit order.
    pub const ALL: [(u64, &str); 22] = [
        (TIX_ABSENT, "tix_absent"),
        (SPOT_MISSING, "spot_missing"),
        (AFTER_INDEX_CLOSE, "after_index_close"),
        (SPOT_UNAVAILABLE, "spot_unavailable"),
        (PEER_MINUTE_ABSENT, "peer_minute_absent"),
        (PEER_UNAVAILABLE, "peer_unavailable"),
        (BODY_NOT_KEPT, "body_not_kept"),
        (SOURCE_CHANGED_SINCE_IMPORT, "source_changed_since_import"),
        (ARCHIVE_DAY_ON_CLOSED_DAY, "archive_day_on_closed_day"),
        (V3_MANIFEST_DISAGREES, "v3_manifest_disagrees"),
        (LINEAGE_UNRECORDED, "lineage_unrecorded"),
        (DETAILS_CAPPED, "details_capped"),
        (NAME_CUT, "name_cut"),
        (DUP_MINUTES, "dup_minutes"),
        (VENDOR_REVISED, "vendor_revised"),
        (EXPIRY_AFTER_CLOSE, "expiry_after_close"),
        (QUALITY_DIFFERS, "quality_differs"),
        (DEFERRED_PENDING, "deferred_pending"),
        (DAYCHECK_DIFFERS, "daycheck_differs"),
        (SERVE_PATH_DIFFERS, "serve_path_differs"),
        (LINEAGE_UNCHECKED, "lineage_unchecked"),
        (SOURCE_CRC_UNRECORDED, "source_crc_unrecorded"),
    ];

    /// Every bit version 1 names. A writer refuses any other; a reader keeps
    /// it.
    pub const KNOWN: u64 = (1 << 22) - 1;

    /// The bits of `bits` this build does not name: a later build's
    /// appended facts, kept by every reader.
    #[must_use]
    pub const fn unnamed(bits: u64) -> u64 {
        bits & !KNOWN
    }

    /// The word of every bit set in `bits`, in bit order: a named bit's
    /// word, and `flag_<n>` for a bit this build does not name. Never
    /// omits a set bit. At most 64 words.
    #[must_use]
    pub fn words(bits: u64) -> Vec<Cow<'static, str>> {
        let mut out = Vec::with_capacity(usize::try_from(bits.count_ones()).unwrap_or(64));
        for at in 0..64u32 {
            let bit = 1u64 << at;
            if bits & bit == 0 {
                continue;
            }
            // `ALL` is in bit order, entry i naming bit i, so the word is one
            // index away (`format_v1_holds_every_fact_it_names` pins it).
            let named = usize::try_from(at)
                .ok()
                .and_then(|i| ALL.get(i))
                .filter(|(b, _)| *b == bit)
                .map(|(_, word)| *word);
            out.push(named.map_or_else(|| Cow::Owned(format!("flag_{at}")), Cow::Borrowed));
        }
        out
    }
}

// ------------------------------------------------------- small enumerations --

/// Declares a one-byte enumeration of the format with its codes and words.
macro_rules! byte_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vm:meta])* $variant:ident = $code:literal => $word:literal,)+ }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vm])* $variant,)+
        }

        impl $name {
            /// Every value, in code order.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// The byte on disk.
            #[must_use]
            pub const fn code(self) -> u8 {
                match self {
                    $(Self::$variant => $code,)+
                }
            }

            /// The value a byte names, or `None` for one this build does not know.
            #[must_use]
            pub const fn from_code(code: u8) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// The word the routes and the documents use.
            #[must_use]
            pub const fn word(self) -> &'static str {
                match self {
                    $(Self::$variant => $word,)+
                }
            }
        }
    };
}

byte_enum! {
    /// Which pass wrote a generation (header offset 146).
    Mode {
        /// Inline, on the import's own workers.
        Inline = 1 => "inline",
        /// A cold verification from the source again.
        Cold = 2 => "cold",
        /// The completeness sweep.
        Completeness = 3 => "completeness",
        /// The deferred cross-kind pass.
        Deferred = 4 => "deferred",
        /// An import with verification off.
        UnverifiedImport = 5 => "unverified_import",
        /// A Zerodha pull run's merge.
        ZerodhaMerge = 6 => "zerodha_merge",
        /// A Zerodha cold verification.
        ZerodhaCold = 7 => "zerodha_cold",
    }
}

byte_enum! {
    /// What a record's unit counts are in (header offset 15).
    Unit {
        /// Seconds of the day.
        Second = 0 => "second",
        /// Minutes of the day.
        Minute = 1 => "minute",
        /// Days.
        Day = 2 => "day",
    }
}

byte_enum! {
    /// How a day ended (day block offset 4). A partial state's worst code is
    /// at least [`Code::DayPartial`] (D-4854).
    DayState {
        /// Every wanted file was reached.
        Complete = 0 => "complete",
        /// The run was stopped.
        Stopped = 1 => "stopped",
        /// The source went away.
        SourceLost = 2 => "source_lost",
        /// The disk filled.
        DiskFull = 3 => "disk_full",
        /// The census refused.
        Census = 4 => "census",
        /// The held-back set filled.
        HeldBackCap = 5 => "held_back_cap",
        /// The day was not imported.
        NotImported = 6 => "not_imported",
        /// The import ran with verification off.
        VerifyOff = 7 => "verify_off",
    }
}

impl DayState {
    /// Whether the day closed before every wanted file was reached.
    #[must_use]
    pub const fn is_partial(self) -> bool {
        matches!(
            self,
            Self::Stopped | Self::SourceLost | Self::DiskFull | Self::Census | Self::HeldBackCap
        )
    }
}

byte_enum! {
    /// What the journal's close of the day said (day block offset 3).
    JournalOutcome {
        /// No close.
        None = 0 => "none",
        /// `done`.
        Done = 1 => "done",
        /// `done`, with refusals.
        DoneWithRefusals = 2 => "done_with_refusals",
        /// `incomplete`.
        Incomplete = 3 => "incomplete",
        /// `refused`.
        Refused = 4 => "refused",
    }
}

byte_enum! {
    /// Whether the source holds the day (day block offset 1).
    SourceHeld {
        /// It does not.
        No = 0 => "no",
        /// It does.
        Yes = 1 => "yes",
        /// Not known.
        Unknown = 2 => "unknown",
    }
}

byte_enum! {
    /// What the `_v3` extraction manifest says (day block offset 2).
    V3Manifest {
        /// Absent, or its variable unset.
        Absent = 0 => "absent",
        /// It agrees.
        Agrees = 1 => "agrees",
        /// It disagrees.
        Disagrees = 2 => "disagrees",
        /// It could not be checked.
        Unverified = 3 => "unverified",
    }
}

// ------------------------------------------------------------ the series --

/// The feed a verdict family is filed under: the bar store's own vendor, so
/// a third feed is a row of `Vendor::ALL` and no change here (RA-243;
/// D-4923).
pub type Feed = Vendor;

/// The byte a feed is filed under (image offset 12): one more than the
/// vendor's position in `Vendor::ALL`, which is appended to and never
/// inserted into. `None` for a vendor the table does not hold.
#[must_use]
pub fn feed_code(feed: Feed) -> Option<u8> {
    let at = Vendor::ALL.iter().position(|vendor| *vendor == feed)?;
    u8::try_from(at.checked_add(1)?).ok()
}

/// The feed a byte names.
#[must_use]
pub fn feed_of(code: u8) -> Option<Feed> {
    let at = usize::from(code).checked_sub(1)?;
    Vendor::ALL.get(at).copied()
}

/// The byte an exchange is filed under (image offset 14). An exhaustive
/// match: an exchange added to `core` does not compile until it has a byte.
#[must_use]
pub const fn exchange_code(exchange: Exchange) -> u8 {
    match exchange {
        Exchange::Nse => 1,
        Exchange::Bse => 2,
    }
}

/// The exchange a byte names.
#[must_use]
pub const fn exchange_of(code: u8) -> Option<Exchange> {
    match code {
        1 => Some(Exchange::Nse),
        2 => Some(Exchange::Bse),
        _ => None,
    }
}

/// The byte a segment is filed under (image offset 13).
#[must_use]
pub const fn segment_code(segment: Segment) -> u8 {
    match segment {
        Segment::Index => 1,
        Segment::Cash => 2,
        Segment::Fno => 3,
    }
}

/// The segment a byte names.
#[must_use]
pub const fn segment_of(code: u8) -> Option<Segment> {
    match code {
        1 => Some(Segment::Index),
        2 => Some(Segment::Cash),
        3 => Some(Segment::Fno),
        _ => None,
    }
}

/// The unit a timeframe's counts are in: the largest of day, minute and
/// second that divides it. Seconds for 1s, minutes for every minute rung,
/// days for 1day, by arithmetic on the timeframe's own length, so a rung the
/// store learns later has a unit without a row here.
#[must_use]
pub const fn unit_of(timeframe: Timeframe) -> Unit {
    let secs = timeframe.secs();
    if secs.is_multiple_of(86_400) {
        Unit::Day
    } else if secs.is_multiple_of(60) {
        Unit::Minute
    } else {
        Unit::Second
    }
}

/// One verdict family: a feed, an exchange, a segment and a timeframe under
/// a store root.
#[derive(Debug, Clone, Copy)]
pub struct Series<'a> {
    /// The store root (the directory holding `bars/`).
    pub root: &'a Path,
    /// The feed.
    pub feed: Feed,
    /// The exchange, whose calendar the day records answer to.
    pub exchange: Exchange,
    /// The segment.
    pub segment: Segment,
    /// The timeframe.
    pub timeframe: Timeframe,
}

impl Series<'_> {
    /// `<root>/verify/<feed>/<EXCHANGE>/<SEGMENT>/<tf>`.
    #[must_use]
    pub fn dir(&self) -> PathBuf {
        self.root
            .join("verify")
            .join(self.feed.as_str())
            .join(self.exchange.as_str())
            .join(self.segment.as_str())
            .join(self.timeframe.as_str())
    }

    /// The year index, `<dir>/<yyyy>.vdi`.
    #[must_use]
    pub fn year_index(&self, year: u16) -> PathBuf {
        self.dir().join(format!("{year:04}.vdi"))
    }

    /// The commit lock, `<dir>/<yyyy>.vdi.lock`.
    #[must_use]
    pub fn lock_path(&self, year: u16) -> PathBuf {
        self.dir().join(format!("{year:04}.vdi.lock"))
    }

    /// One generation's image, `<dir>/<yyyy>/<yyyy-mm-dd>.g<NNNNNNNNNN>.vdt`:
    /// the generation zero-padded to 10 digits, the width of `u32::MAX`, so
    /// every generation has exactly one name and one day's names sort in
    /// generation order (D-4844).
    #[must_use]
    pub fn image_path(&self, day: Day, generation: u32) -> PathBuf {
        self.dir()
            .join(format!("{:04}", day.year()))
            .join(format!("{day}.g{generation:010}.vdt"))
    }

    /// The feed's byte for every image header, or the refusal.
    fn feed_byte(&self) -> Result<u8, VerdictError> {
        feed_code(self.feed).ok_or(VerdictError::FeedUnknown {
            feed: self.feed.as_str(),
        })
    }
}

// ------------------------------------------------------------------- days --

/// Days from 1970-01-01 to `(year, month, day)` of the proleptic Gregorian
/// calendar (the civil-from-days inverse, by integer arithmetic only).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days since 1970-01-01, as the image header stores them.
fn epoch_day(day: Day) -> Result<u32, VerdictError> {
    let days = days_from_civil(
        i64::from(day.year()),
        i64::from(day.month()),
        i64::from(day.day()),
    );
    u32::try_from(days).map_err(|_| VerdictError::DayOutOfRange { day })
}

/// The day's position in its year, 0 for 1 January.
fn day_of_year(day: Day) -> usize {
    let year = i64::from(day.year());
    let at = days_from_civil(year, i64::from(day.month()), i64::from(day.day()));
    usize::try_from(at - days_from_civil(year, 1, 1)).unwrap_or(0)
}

// -------------------------------------------------------------------- keys --

/// A record's key: an instrument key, or a refused file's stem.
///
/// The hash is BLAKE3 of the WHOLE text, first 8 bytes little-endian; the
/// record keeps at most [`KEY_LEN`] bytes of the text. Two keys are equal
/// when both the hash and the kept text are.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// The kept text, at most [`KEY_LEN`] bytes, never empty, no NUL.
    text: String,
    /// BLAKE3 of the whole text, first 8 bytes, little-endian.
    hash: u64,
    /// The whole text, when the kept text is a cut of it.
    whole: Option<String>,
}

impl Key {
    /// An instrument key. Never cut: a longer one is refused (D-4844).
    ///
    /// # Errors
    ///
    /// [`VerdictError::KeyTooLong`] past [`KEY_LEN`] bytes,
    /// [`VerdictError::KeyInvalid`] for an empty key or one holding NUL.
    pub fn instrument(text: &str) -> Result<Self, VerdictError> {
        check_text(text)?;
        if text.len() > KEY_LEN {
            return Err(VerdictError::KeyTooLong { len: text.len() });
        }
        Ok(Self {
            text: text.to_owned(),
            hash: key_hash(text.as_bytes()),
            whole: None,
        })
    }

    /// A refused file's stem as listed. One longer than [`KEY_LEN`] bytes
    /// keeps its first bytes up to a character boundary, its record carries
    /// [`flags::NAME_CUT`], and the whole name goes in detail [`DETAIL_NAME`].
    ///
    /// # Errors
    ///
    /// [`VerdictError::KeyInvalid`] for an empty stem or one holding NUL.
    pub fn stem(text: &str) -> Result<Self, VerdictError> {
        check_text(text)?;
        let hash = key_hash(text.as_bytes());
        if text.len() <= KEY_LEN {
            return Ok(Self {
                text: text.to_owned(),
                hash,
                whole: None,
            });
        }
        let mut end = KEY_LEN;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Ok(Self {
            text: text.get(..end).unwrap_or_default().to_owned(),
            hash,
            whole: Some(text.to_owned()),
        })
    }

    /// The kept text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The whole text, when the kept text is a cut of it.
    #[must_use]
    pub fn whole(&self) -> Option<&str> {
        self.whole.as_deref()
    }

    /// The key hash.
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }

    /// The table's tag: bits 32 to 63 of the hash.
    fn tag(&self) -> u32 {
        tag_of(self.hash)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.whole.as_deref().unwrap_or(&self.text))
    }
}

/// BLAKE3 of `text`, first 8 bytes, little-endian.
#[must_use]
pub fn key_hash(text: &[u8]) -> u64 {
    let digest = blake3::hash(text);
    u64::from_le_bytes(take::<8>(&digest, 0))
}

/// Bits 32 to 63 of a key hash.
fn tag_of(hash: u64) -> u32 {
    u32::try_from(hash >> 32).unwrap_or(u32::MAX)
}

/// A key's home slot in a table of `capacity` slots (a power of two).
fn home_of(hash: u64, capacity: usize) -> usize {
    let mask = u64::try_from(capacity.saturating_sub(1)).unwrap_or(u64::MAX);
    usize::try_from(hash & mask).unwrap_or(0)
}

/// An empty text, or one holding NUL (which the zero padding could not tell
/// from the end), is refused.
fn check_text(text: &str) -> Result<(), VerdictError> {
    if text.is_empty() || text.bytes().any(|b| b == 0) {
        return Err(VerdictError::KeyInvalid);
    }
    Ok(())
}

// ----------------------------------------------------------------- records --

/// What verification found for one key: every numeric field of a record but
/// its key, its run and its detail offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// The verdict code.
    pub code: Code,
    /// The rule set the unit was compared under (0 = not applicable).
    pub rule_set: u8,
    /// The reason, opaque to `store` (`pull`'s `REASONS`).
    pub reason: u16,
    /// Flags, [`flags`].
    pub flags: u64,
    /// Source units (oracle seconds or minutes).
    pub source_units: u32,
    /// Store units.
    pub store_units: u32,
    /// Units missing from the store.
    pub missing: u32,
    /// Units the store holds extra.
    pub extra: u32,
    /// Units whose price differs.
    pub value_diff: u32,
    /// Units whose volume differs.
    pub volume_diff: u32,
    /// Units whose open interest differs.
    pub oi_diff: u32,
    /// The first differing unit of the day, or [`NONE`].
    pub first_diff: u32,
    /// The month generation read (0 = the original file).
    pub month_generation: u32,
    /// The day's first stored row in the month, or [`NONE`].
    pub first_row: u32,
    /// The day's stored row count.
    pub rows: u32,
    /// The first stored unit of the day, or [`NONE`].
    pub first_unit: u32,
    /// The last stored unit of the day, or [`NONE`].
    pub last_unit: u32,
}

impl Facts {
    /// The facts of a code with every count zero and every position none.
    #[must_use]
    pub const fn of(code: Code) -> Self {
        Self {
            code,
            rule_set: 0,
            reason: 0,
            flags: 0,
            source_units: 0,
            store_units: 0,
            missing: 0,
            extra: 0,
            value_diff: 0,
            volume_diff: 0,
            oi_diff: 0,
            first_diff: NONE,
            month_generation: 0,
            first_row: NONE,
            rows: 0,
            first_unit: NONE,
            last_unit: NONE,
        }
    }
}

/// A detail entry: an opaque kind (`pull`'s `DETAILS`) and its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The kind, never 0.
    pub kind: u16,
    /// The payload, at most `u16::MAX` bytes.
    pub payload: Vec<u8>,
}

/// One key's verdict as a writer hands it to [`commit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The key.
    pub key: Key,
    /// The facts.
    pub facts: Facts,
    /// Its details, in the order given.
    pub details: Vec<Detail>,
}

/// One record as it reads back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The key (its kept text and hash; a cut key's whole name is in its
    /// detail [`DETAIL_NAME`]).
    pub key: Key,
    /// The run that wrote this record; a carried record keeps its own.
    pub run: u32,
    /// The facts.
    pub facts: Facts,
    /// Its first detail entry's offset in the detail section, plus one (0 =
    /// none).
    pub detail_at: u32,
}

/// What a day image says about the day as a whole (the 256-byte day block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayBlock {
    /// The calendar's kind of the day, opaque to `store`.
    pub calendar_kind: u8,
    /// Whether the source holds the day.
    pub source_held: SourceHeld,
    /// What the `_v3` manifest says.
    pub v3: V3Manifest,
    /// The journal's close.
    pub journal: JournalOutcome,
    /// How the day ended.
    pub state: DayState,
    /// The worst code. A writer states the worst of the day's own findings;
    /// [`commit`] folds in every record and, for a partial state,
    /// [`Code::DayPartial`].
    pub worst: Code,
    /// The journal close's reason, opaque to `store`.
    pub close_reason: u16,
    /// Day flags, [`flags`].
    pub flags: u64,
    /// Listing entries.
    pub listing_entries: u32,
    /// Files read.
    pub files_read: u32,
    /// Files skipped.
    pub skipped: u32,
    /// Files refused.
    pub refused: u32,
    /// Wanted files the run never read.
    pub not_reached: u32,
    /// The skip census, 8 counts.
    pub skip_census: [u32; 8],
    /// Journal seconds (the close's own filter).
    pub journal_seconds: u64,
    /// Stored seconds (the same scope).
    pub stored_seconds: u64,
    /// Oracle seconds.
    pub oracle_seconds: u64,
    /// Swept shares absent.
    pub shares_absent: u32,
}

impl DayBlock {
    /// A complete day whose own findings are `worst`, every count zero.
    #[must_use]
    pub const fn of(worst: Code) -> Self {
        Self {
            calendar_kind: 0,
            source_held: SourceHeld::Unknown,
            v3: V3Manifest::Absent,
            journal: JournalOutcome::None,
            state: DayState::Complete,
            worst,
            close_reason: 0,
            flags: 0,
            listing_entries: 0,
            files_read: 0,
            skipped: 0,
            refused: 0,
            not_reached: 0,
            skip_census: [0; 8],
            journal_seconds: 0,
            stored_seconds: 0,
            oracle_seconds: 0,
            shares_absent: 0,
        }
    }
}

/// An image header as it reads back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageHeader {
    /// The feed.
    pub feed: Feed,
    /// The exchange.
    pub exchange: Exchange,
    /// The segment.
    pub segment: Segment,
    /// The timeframe.
    pub timeframe: Timeframe,
    /// The unit of the counts.
    pub unit: Unit,
    /// Days since 1970-01-01.
    pub epoch_day: u32,
    /// This image's generation.
    pub generation: u32,
    /// Records.
    pub records: u32,
    /// Table capacity (a power of two).
    pub capacity: u32,
    /// Records per verdict code 0 to 23.
    pub counts: [u32; CODE_SLOTS],
    /// The run that wrote this generation.
    pub run: u64,
    /// When it was written (unix seconds).
    pub written_at: u64,
    /// The verifier version.
    pub verifier: u16,
    /// The pass that wrote it.
    pub mode: Mode,
    /// The day's rule set (0 = not applicable).
    pub rule_set: u8,
    /// The generation its carried records came from (0 = none).
    pub previous: u32,
    /// Records carried unchanged from it.
    pub carried: u32,
    /// The origin digest prefix.
    pub origin: [u8; 16],
    /// The detail section's absolute offset (0 = none).
    pub detail_offset: u32,
    /// The detail section's bytes.
    pub detail_bytes: u32,
    /// The detail section's entries.
    pub detail_entries: u32,
}

impl ImageHeader {
    /// The first record ordinal of `code` and how many records carry it:
    /// one code's records are one contiguous slice. Codes 2 to 5 and 7 to 9
    /// (the mismatches) are two slices, because code 6 sits between them.
    #[must_use]
    pub fn slice_of(&self, code: Code) -> (u32, u32) {
        let at = usize::from(code.code());
        let start = self
            .counts
            .iter()
            .take(at)
            .fold(0u32, |sum, n| sum.saturating_add(*n));
        (start, self.counts.get(at).copied().unwrap_or(0))
    }

    /// Every mismatch of the day as at most two contiguous slices of
    /// `(first ordinal, records)`: codes 2 to 5, then codes 7 to 9. Code 6
    /// (`source_refused`, not a mismatch) sorts between them. O(1) from the
    /// header's counts; reading a page of either is O(page)
    /// (`crates/store/tests/verdict.rs`,
    /// `one_codes_records_are_one_contiguous_slice`, IA-VD-06).
    #[must_use]
    pub fn mismatch_slices(&self) -> [(u32, u32); 2] {
        let span = |from: Code, to: Code| {
            let (start, _) = self.slice_of(from);
            let (last, n) = self.slice_of(to);
            (start, last.saturating_add(n).saturating_sub(start))
        };
        [
            span(Code::ValueMismatch, Code::CountMismatch),
            span(Code::StoreWithoutSource, Code::StoreUnreadable),
        ]
    }
}

// ------------------------------------------------------------------ errors --

/// Every refusal of the verdict store, each named.
#[derive(Debug)]
#[non_exhaustive]
pub enum VerdictError {
    /// `key_too_long`: an instrument key longer than [`KEY_LEN`] bytes; never cut.
    KeyTooLong {
        /// Its length in bytes.
        len: usize,
    },
    /// `key_invalid`: an empty key, or one holding NUL.
    KeyInvalid,
    /// `duplicate_key`: one generation names a key twice.
    DuplicateKey {
        /// The key.
        key: String,
    },
    /// `table_too_large`: no table of at most [`MAX_CAPACITY`] slots holds
    /// the day's keys within [`MAX_PROBE`] slots with no repeated tag. A day
    /// over the record ceiling (L-IA-33) is refused whole, never cut.
    TableTooLarge {
        /// The day's records.
        records: usize,
    },
    /// `run_id_overflow`: a run id past `u32::MAX` (year 2106).
    RunIdOverflow {
        /// The run id.
        run: u64,
    },
    /// `generation_collision`: [`MAX_LINK_TRIES`] generations past the
    /// committed one were all already linked.
    GenerationCollision {
        /// The image path of the last generation tried.
        path: PathBuf,
    },
    /// `flag_unknown`: a writer was handed a flag bit version 1 does not
    /// name. Only a writer refuses it; a reader keeps it (D-4844).
    FlagUnknown {
        /// The unknown bits.
        bits: u64,
    },
    /// `detail_invalid`: a detail of kind 0 or longer than `u16::MAX` bytes.
    DetailInvalid {
        /// The kind.
        kind: u16,
        /// The payload length.
        len: usize,
    },
    /// `day_out_of_range`: a day before 1970-01-01.
    DayOutOfRange {
        /// The day.
        day: Day,
    },
    /// `feed_unknown`: a vendor `Vendor::ALL` does not hold, so it has no
    /// feed byte.
    FeedUnknown {
        /// The vendor's path segment.
        feed: &'static str,
    },
    /// `staging_linked`, `staging_not_regular` or `staging_contended`: a
    /// writer's temporary could not be created without opening something
    /// that was already there ([`crate::staging`]; RA-290). Nothing was
    /// truncated or written through.
    Staging(StagingError),
    /// `lock_stalled`: the commit lock stayed held for the whole wait. The
    /// day is "verdict pending"; nothing was published (D-4924).
    LockStalled {
        /// The lock file.
        path: PathBuf,
        /// How long the commit waited, in milliseconds.
        waited_ms: u64,
        /// How many times it tried the lock: one, and one more per
        /// [`LOCK_STEP`] of the wait.
        tries: u32,
    },
    /// `written_by_newer_build`: a value this build does not know, which a
    /// later format addition wrote. Refused, never read as something else.
    Unknown {
        /// The file.
        path: PathBuf,
        /// What held it.
        what: &'static str,
        /// The value.
        value: u64,
    },
    /// `index_short`: a year index shorter than [`INDEX_LEN`]; no file this
    /// protocol links is.
    IndexShort {
        /// The file.
        path: PathBuf,
        /// Its length.
        len: u64,
    },
    /// `index_unreadable`: neither year-index slot is valid.
    IndexUnreadable {
        /// The file.
        path: PathBuf,
    },
    /// `image_missing`: the year index commits a generation whose image is
    /// absent.
    ImageMissing {
        /// The image path.
        path: PathBuf,
    },
    /// `image_corrupt`: an image fails a check, named.
    ImageCorrupt {
        /// The image path.
        path: PathBuf,
        /// Which check.
        why: String,
    },
    /// `trailer_mismatch`: an image's bytes do not hash to its BLAKE3
    /// trailer, so at least one byte changed since it was linked. Only
    /// [`check_trailer`] reads the trailer; a lookup never does.
    TrailerMismatch {
        /// The image path.
        path: PathBuf,
    },
    /// `record_corrupt`: a record's CRC-32C does not match.
    RecordCorrupt {
        /// The image path.
        path: PathBuf,
        /// The record's ordinal.
        ordinal: u32,
    },
    /// `symlinked`: a verdict file is a symbolic link.
    Symlinked {
        /// The path.
        path: PathBuf,
    },
    /// `io`: the host refused an operation.
    Io {
        /// The path.
        path: PathBuf,
        /// What was being done.
        action: &'static str,
        /// The host's refusal.
        source: io::Error,
    },
    /// A crash point armed by a unit test was reached: the writer stopped
    /// there, as a killed process would.
    #[cfg(test)]
    Crashed {
        /// The point.
        at: &'static str,
    },
}

impl VerdictError {
    /// The refusal's word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::KeyTooLong { .. } => "key_too_long",
            Self::KeyInvalid => "key_invalid",
            Self::DuplicateKey { .. } => "duplicate_key",
            Self::TableTooLarge { .. } => "table_too_large",
            Self::RunIdOverflow { .. } => "run_id_overflow",
            Self::GenerationCollision { .. } => "generation_collision",
            Self::FlagUnknown { .. } => "flag_unknown",
            Self::DetailInvalid { .. } => "detail_invalid",
            Self::DayOutOfRange { .. } => "day_out_of_range",
            Self::FeedUnknown { .. } => "feed_unknown",
            Self::Staging(why) => why.word(),
            Self::LockStalled { .. } => "lock_stalled",
            Self::Unknown { .. } => "written_by_newer_build",
            Self::IndexShort { .. } => "index_short",
            Self::IndexUnreadable { .. } => "index_unreadable",
            Self::ImageMissing { .. } => "image_missing",
            Self::ImageCorrupt { .. } => "image_corrupt",
            Self::TrailerMismatch { .. } => "trailer_mismatch",
            Self::RecordCorrupt { .. } => "record_corrupt",
            Self::Symlinked { .. } => "symlinked",
            Self::Io { .. } => "io",
            #[cfg(test)]
            Self::Crashed { .. } => "crashed",
        }
    }
}

impl fmt::Display for VerdictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = self.word();
        match self {
            Self::KeyTooLong { len } => write!(
                f,
                "{word}: an instrument key of {len} bytes does not fit the {KEY_LEN}-byte key field, and a key is never cut"
            ),
            Self::KeyInvalid => write!(f, "{word}: a key is empty or holds a NUL byte"),
            Self::DuplicateKey { key } => write!(f, "{word}: one generation names {key} twice"),
            Self::TableTooLarge { records } => write!(
                f,
                "{word}: no table of at most {MAX_CAPACITY} slots holds {records} keys within {MAX_PROBE} slots of home with no repeated tag (format version 1's record ceiling); nothing was written and the day's verdict is pending"
            ),
            Self::RunIdOverflow { run } => {
                write!(
                    f,
                    "{word}: run id {run} does not fit a record's u32 run field"
                )
            }
            Self::GenerationCollision { path } => write!(
                f,
                "{word}: {MAX_LINK_TRIES} generations up to {} were all already linked; none is overwritten",
                path.display()
            ),
            Self::FlagUnknown { bits } => {
                write!(
                    f,
                    "{word}: flag bits {bits:#x} are not named by format version 1, and a writer never writes a fact it cannot name"
                )
            }
            Self::DetailInvalid { kind, len } => write!(
                f,
                "{word}: a detail of kind {kind} and {len} bytes (kind 0 is unset; a payload holds at most 65535 bytes)"
            ),
            Self::DayOutOfRange { day } => {
                write!(
                    f,
                    "{word}: {day} is before 1970-01-01, which a day field cannot hold"
                )
            }
            Self::FeedUnknown { feed } => {
                write!(
                    f,
                    "{word}: vendor {feed} is not in the store's vendor table"
                )
            }
            Self::Staging(why) => write!(f, "{why}"),
            Self::LockStalled {
                path,
                waited_ms,
                tries,
            } => write!(
                f,
                "{word}: {} stayed held through {tries} tries over {waited_ms} ms; the commit was not made",
                path.display()
            ),
            Self::Unknown { path, what, value } => write!(
                f,
                "{word}: {} holds {what} {value}, which this build does not know",
                path.display()
            ),
            Self::IndexShort { path, len } => write!(
                f,
                "{word}: {} is {len} bytes, shorter than the {INDEX_LEN} a year index is born with",
                path.display()
            ),
            Self::IndexUnreadable { path } => write!(
                f,
                "{word}: neither slot of {} is a valid year-index slot",
                path.display()
            ),
            Self::ImageMissing { path } => write!(
                f,
                "{word}: the year index commits {}, which does not exist",
                path.display()
            ),
            Self::ImageCorrupt { path, why } => write!(f, "{word}: {}: {why}", path.display()),
            Self::TrailerMismatch { path } => write!(
                f,
                "{word}: {} does not hash to its BLAKE3 trailer; a byte changed after it was linked",
                path.display()
            ),
            Self::RecordCorrupt { path, ordinal } => write!(
                f,
                "{word}: record {ordinal} of {} fails its CRC-32C",
                path.display()
            ),
            Self::Symlinked { path } => write!(
                f,
                "{word}: {} is a symbolic link, which the verdict store never follows",
                path.display()
            ),
            Self::Io {
                path,
                action,
                source,
            } => write!(f, "{word}: {action} {}: {source}", path.display()),
            #[cfg(test)]
            Self::Crashed { at } => write!(f, "{word}: the writer stopped at {at}"),
        }
    }
}

impl std::error::Error for VerdictError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Staging(why) => Some(why),
            _ => None,
        }
    }
}

/// [`VerdictError::Io`].
fn io_error(path: &Path, action: &'static str, source: io::Error) -> VerdictError {
    VerdictError::Io {
        path: path.to_path_buf(),
        action,
        source,
    }
}

/// [`VerdictError::ImageCorrupt`].
fn corrupt(path: &Path, why: impl Into<String>) -> VerdictError {
    VerdictError::ImageCorrupt {
        path: path.to_path_buf(),
        why: why.into(),
    }
}

// -------------------------------------------------------- byte primitives --

/// Writes `bytes` into `out` at `at`; every call site passes an offset inside
/// a buffer sized for it.
fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
        dst.copy_from_slice(bytes);
    }
}

/// `N` bytes of `bytes` from `at`, or zeros past its end.
fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(run) = bytes.get(at..at.saturating_add(N)) {
        out.copy_from_slice(run);
    }
    out
}

fn u8_at(bytes: &[u8], at: usize) -> u8 {
    let [byte] = take::<1>(bytes, at);
    byte
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(take::<2>(bytes, at))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(take::<4>(bytes, at))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(take::<8>(bytes, at))
}

/// CRC-32C of `bytes[..end]`.
fn crc_to(bytes: &[u8], end: usize) -> u32 {
    crc32c(bytes.get(..end).unwrap_or_default())
}

/// Whether every byte of `bytes[from..to]` is zero.
fn zero(bytes: &[u8], from: usize, to: usize) -> bool {
    bytes
        .get(from..to)
        .is_some_and(|run| run.iter().all(|b| *b == 0))
}

/// `usize` to `u32`, refusing a value that does not fit.
fn u32_of(value: usize, path: &Path, what: &str) -> Result<u32, VerdictError> {
    u32::try_from(value).map_err(|_| corrupt(path, format!("{what} {value} does not fit a u32")))
}

/// `u64` to `usize`; lossless on every target this crate builds for.
fn usize_of(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// A file offset from a `usize`; lossless on every target this crate builds
/// for.
fn off(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

// ------------------------------------------------------------ the record --

/// The 160 bytes of one record.
fn encode_record(key: &Key, run: u32, facts: &Facts, detail_at: u32) -> [u8; RECORD_STRIDE] {
    let mut out = [0u8; RECORD_STRIDE];
    put(&mut out, 0, &key.hash.to_le_bytes());
    put(&mut out, 8, key.text.as_bytes());
    put(&mut out, 72, &[facts.code.code(), facts.rule_set]);
    put(&mut out, 74, &facts.reason.to_le_bytes());
    put(&mut out, 76, &run.to_le_bytes());
    put(&mut out, 80, &facts.flags.to_le_bytes());
    for (at, value) in [
        (88, facts.source_units),
        (92, facts.store_units),
        (96, facts.missing),
        (100, facts.extra),
        (104, facts.value_diff),
        (108, facts.volume_diff),
        (112, facts.oi_diff),
        (116, facts.first_diff),
        (120, detail_at),
        (124, facts.month_generation),
        (128, facts.first_row),
        (132, facts.rows),
        (136, facts.first_unit),
        (140, facts.last_unit),
    ] {
        put(&mut out, at, &value.to_le_bytes());
    }
    let crc = crc_to(&out, 156);
    put(&mut out, 156, &crc.to_le_bytes());
    out
}

/// One record from its 160 bytes, its CRC checked first.
fn decode_record(bytes: &[u8], path: &Path, ordinal: u32) -> Result<Record, VerdictError> {
    if bytes.len() != RECORD_STRIDE || crc_to(bytes, 156) != u32_at(bytes, 156) {
        return Err(VerdictError::RecordCorrupt {
            path: path.to_path_buf(),
            ordinal,
        });
    }
    let field = bytes.get(8..8 + KEY_LEN).unwrap_or_default();
    let len = field.iter().take_while(|b| **b != 0).count();
    let text = std::str::from_utf8(field.get(..len).unwrap_or_default())
        .ok()
        .filter(|t| !t.is_empty() && zero(field, len, KEY_LEN))
        .ok_or_else(|| corrupt(path, format!("record {ordinal}'s key field is not a key")))?;
    let code = u8_at(bytes, 72);
    let code = Code::from_code(code).ok_or_else(|| VerdictError::Unknown {
        path: path.to_path_buf(),
        what: "verdict code",
        value: u64::from(code),
    })?;
    if !zero(bytes, 144, 156) {
        return Err(corrupt(
            path,
            format!("record {ordinal}'s reserved bytes are not zero"),
        ));
    }
    Ok(Record {
        key: Key {
            text: text.to_owned(),
            hash: u64_at(bytes, 0),
            whole: None,
        },
        run: u32_at(bytes, 76),
        facts: Facts {
            code,
            rule_set: u8_at(bytes, 73),
            reason: u16_at(bytes, 74),
            flags: u64_at(bytes, 80),
            source_units: u32_at(bytes, 88),
            store_units: u32_at(bytes, 92),
            missing: u32_at(bytes, 96),
            extra: u32_at(bytes, 100),
            value_diff: u32_at(bytes, 104),
            volume_diff: u32_at(bytes, 108),
            oi_diff: u32_at(bytes, 112),
            first_diff: u32_at(bytes, 116),
            month_generation: u32_at(bytes, 124),
            first_row: u32_at(bytes, 128),
            rows: u32_at(bytes, 132),
            first_unit: u32_at(bytes, 136),
            last_unit: u32_at(bytes, 140),
        },
        detail_at: u32_at(bytes, 120),
    })
}

// --------------------------------------------------------- the day block --

/// The 256 bytes of a day block.
fn encode_day(day: &DayBlock) -> [u8; DAY_BLOCK_LEN] {
    let mut out = [0u8; DAY_BLOCK_LEN];
    put(
        &mut out,
        0,
        &[
            day.calendar_kind,
            day.source_held.code(),
            day.v3.code(),
            day.journal.code(),
            day.state.code(),
            day.worst.code(),
        ],
    );
    put(&mut out, 6, &day.close_reason.to_le_bytes());
    put(&mut out, 8, &day.flags.to_le_bytes());
    for (at, value) in [
        (16, day.listing_entries),
        (20, day.files_read),
        (24, day.skipped),
        (28, day.refused),
        (32, day.not_reached),
    ] {
        put(&mut out, at, &value.to_le_bytes());
    }
    for (i, count) in day.skip_census.iter().enumerate() {
        put(&mut out, 36 + 4 * i, &count.to_le_bytes());
    }
    put(&mut out, 68, &day.journal_seconds.to_le_bytes());
    put(&mut out, 76, &day.stored_seconds.to_le_bytes());
    put(&mut out, 84, &day.oracle_seconds.to_le_bytes());
    put(&mut out, 92, &day.shares_absent.to_le_bytes());
    let crc = crc_to(&out, 252);
    put(&mut out, 252, &crc.to_le_bytes());
    out
}

/// A one-byte enumeration's value, or the refusal naming it.
fn known<T>(value: Option<T>, path: &Path, what: &'static str, raw: u8) -> Result<T, VerdictError> {
    value.ok_or_else(|| VerdictError::Unknown {
        path: path.to_path_buf(),
        what,
        value: u64::from(raw),
    })
}

/// A day block from its 256 bytes, its CRC checked first.
fn decode_day(bytes: &[u8], path: &Path) -> Result<DayBlock, VerdictError> {
    if bytes.len() != DAY_BLOCK_LEN || crc_to(bytes, 252) != u32_at(bytes, 252) {
        return Err(corrupt(path, "the day block fails its CRC-32C"));
    }
    if !zero(bytes, 96, 252) {
        return Err(corrupt(path, "the day block's reserved bytes are not zero"));
    }
    let raw = |at| u8_at(bytes, at);
    let mut skip_census = [0u32; 8];
    for (i, slot) in skip_census.iter_mut().enumerate() {
        *slot = u32_at(bytes, 36 + 4 * i);
    }
    Ok(DayBlock {
        calendar_kind: raw(0),
        source_held: known(SourceHeld::from_code(raw(1)), path, "source held", raw(1))?,
        v3: known(V3Manifest::from_code(raw(2)), path, "v3 manifest", raw(2))?,
        journal: known(
            JournalOutcome::from_code(raw(3)),
            path,
            "journal outcome",
            raw(3),
        )?,
        state: known(DayState::from_code(raw(4)), path, "day state", raw(4))?,
        worst: known(Code::from_code(raw(5)), path, "worst code", raw(5))?,
        close_reason: u16_at(bytes, 6),
        flags: u64_at(bytes, 8),
        listing_entries: u32_at(bytes, 16),
        files_read: u32_at(bytes, 20),
        skipped: u32_at(bytes, 24),
        refused: u32_at(bytes, 28),
        not_reached: u32_at(bytes, 32),
        skip_census,
        journal_seconds: u64_at(bytes, 68),
        stored_seconds: u64_at(bytes, 76),
        oracle_seconds: u64_at(bytes, 84),
        shares_absent: u32_at(bytes, 92),
    })
}

// ------------------------------------------------------------ the header --

/// The 256 bytes of an image header, its CRC over bytes 0 to 252.
fn encode_header(header: &ImageHeader) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    put(&mut out, 0, &IMAGE_MAGIC);
    put(&mut out, 8, &IMAGE_VERSION.to_le_bytes());
    put(&mut out, 10, &160u16.to_le_bytes());
    put(
        &mut out,
        12,
        &[
            feed_code(header.feed).unwrap_or(0),
            segment_code(header.segment),
            exchange_code(header.exchange),
            header.unit.code(),
        ],
    );
    put(&mut out, 16, &header.epoch_day.to_le_bytes());
    put(&mut out, 20, &header.generation.to_le_bytes());
    put(&mut out, 24, &header.records.to_le_bytes());
    put(&mut out, 28, &header.capacity.to_le_bytes());
    for (i, count) in header.counts.iter().enumerate() {
        put(&mut out, 32 + 4 * i, &count.to_le_bytes());
    }
    put(&mut out, 128, &header.run.to_le_bytes());
    put(&mut out, 136, &header.written_at.to_le_bytes());
    put(&mut out, 144, &header.verifier.to_le_bytes());
    put(&mut out, 146, &[header.mode.code(), header.rule_set]);
    put(&mut out, 148, &header.previous.to_le_bytes());
    put(&mut out, 152, &header.carried.to_le_bytes());
    put(&mut out, 156, &header.origin);
    put(&mut out, 172, &header.detail_offset.to_le_bytes());
    put(&mut out, 176, &header.detail_bytes.to_le_bytes());
    put(&mut out, 180, &header.detail_entries.to_le_bytes());
    put(&mut out, 184, &header.timeframe.secs().to_le_bytes());
    let crc = crc_to(&out, 252);
    put(&mut out, 252, &crc.to_le_bytes());
    out
}

/// An image header from its 256 bytes: magic, version, stride, CRC and
/// every enumeration checked.
fn decode_header(bytes: &[u8], path: &Path) -> Result<ImageHeader, VerdictError> {
    if take::<8>(bytes, 0) != IMAGE_MAGIC {
        return Err(corrupt(path, "the magic is not BRUTEXVD"));
    }
    let version = u16_at(bytes, 8);
    if version != IMAGE_VERSION {
        return Err(VerdictError::Unknown {
            path: path.to_path_buf(),
            what: "image version",
            value: u64::from(version),
        });
    }
    if u16_at(bytes, 10) != 160 {
        return Err(corrupt(path, "the record stride is not 160"));
    }
    if bytes.len() < HEADER_LEN || crc_to(bytes, 252) != u32_at(bytes, 252) {
        return Err(corrupt(path, "the header fails its CRC-32C"));
    }
    if !zero(bytes, 188, 252) {
        return Err(corrupt(path, "the header's reserved bytes are not zero"));
    }
    let raw = |at| u8_at(bytes, at);
    let mut counts = [0u32; CODE_SLOTS];
    for (i, slot) in counts.iter_mut().enumerate() {
        *slot = u32_at(bytes, 32 + 4 * i);
    }
    let secs = u32_at(bytes, 184);
    let timeframe = Timeframe::from_secs(secs).map_err(|_| VerdictError::Unknown {
        path: path.to_path_buf(),
        what: "timeframe seconds",
        value: u64::from(secs),
    })?;
    let unit = known(Unit::from_code(raw(15)), path, "unit", raw(15))?;
    if unit != unit_of(timeframe) {
        return Err(corrupt(path, "the unit is not the timeframe's"));
    }
    Ok(ImageHeader {
        feed: known(feed_of(raw(12)), path, "feed", raw(12))?,
        exchange: known(exchange_of(raw(14)), path, "exchange", raw(14))?,
        segment: known(segment_of(raw(13)), path, "segment", raw(13))?,
        timeframe,
        unit,
        epoch_day: u32_at(bytes, 16),
        generation: u32_at(bytes, 20),
        records: u32_at(bytes, 24),
        capacity: u32_at(bytes, 28),
        counts,
        run: u64_at(bytes, 128),
        written_at: u64_at(bytes, 136),
        verifier: u16_at(bytes, 144),
        mode: known(Mode::from_code(raw(146)), path, "mode", raw(146))?,
        rule_set: raw(147),
        previous: u32_at(bytes, 148),
        carried: u32_at(bytes, 152),
        origin: take::<16>(bytes, 156),
        detail_offset: u32_at(bytes, 172),
        detail_bytes: u32_at(bytes, 176),
        detail_entries: u32_at(bytes, 180),
    })
}

/// The geometry a header implies, checked: the table's slots, where the
/// records start, and the file's whole length.
struct Geometry {
    capacity: usize,
    records_at: usize,
    len: usize,
}

fn geometry(header: &ImageHeader, path: &Path) -> Result<Geometry, VerdictError> {
    let capacity = usize_of(u64::from(header.capacity));
    let n = usize_of(u64::from(header.records));
    let in_range = matches!(capacity, MIN_CAPACITY..=MAX_CAPACITY);
    if !capacity.is_power_of_two() || !in_range {
        return Err(corrupt(
            path,
            format!("table capacity {capacity} is not a power of two in range"),
        ));
    }
    if n.saturating_mul(2) > capacity {
        return Err(corrupt(
            path,
            format!("{n} records overfill a table of {capacity}"),
        ));
    }
    let counted = header
        .counts
        .iter()
        .fold(0u64, |sum, c| sum.saturating_add(u64::from(*c)));
    if counted != u64::from(header.records) || header.counts.first().copied() != Some(0) {
        return Err(corrupt(
            path,
            "the per-code counts do not sum to the records",
        ));
    }
    let records_at = TABLE_OFFSET + SLOT_LEN * (capacity + OVERFLOW_SLOTS);
    let records_end = records_at + RECORD_STRIDE * n;
    let detail_bytes = usize_of(u64::from(header.detail_bytes));
    let detail_at = usize_of(u64::from(header.detail_offset));
    if detail_bytes > DETAIL_CAP
        || (detail_bytes == 0) != (detail_at == 0)
        || (detail_bytes == 0) != (header.detail_entries == 0)
        || (detail_at != 0 && detail_at != records_end)
    {
        return Err(corrupt(
            path,
            "the detail section's place does not follow the records",
        ));
    }
    Ok(Geometry {
        capacity,
        records_at,
        len: records_end + detail_bytes + TRAILER_LEN,
    })
}

// --------------------------------------------------------------- the table --

/// The table of `hashes` (in record order) at the smallest capacity from
/// `max(16, 2n)` upward whose every key sits within [`MAX_PROBE`] slots of
/// its home with no other occupied slot of that window carrying its tag.
///
/// Returned as the capacity and `c + 7` slots of `(ordinal + 1, tag)`.
///
/// # Errors
///
/// [`VerdictError::TableTooLarge`] when no capacity up to [`MAX_CAPACITY`]
/// does.
pub fn table(hashes: &[u64]) -> Result<(usize, Vec<(u32, u32)>), VerdictError> {
    let n = hashes.len();
    let too_large = VerdictError::TableTooLarge { records: n };
    let mut capacity = n
        .saturating_mul(2)
        .checked_next_power_of_two()
        .unwrap_or(usize::MAX)
        .max(MIN_CAPACITY);
    while capacity <= MAX_CAPACITY {
        if let Some(slots) = place(hashes, capacity) {
            return Ok((capacity, slots));
        }
        capacity = capacity.saturating_mul(2);
    }
    Err(too_large)
}

/// One attempt at `capacity`, or `None` when a key does not fit the rule.
fn place(hashes: &[u64], capacity: usize) -> Option<Vec<(u32, u32)>> {
    let mut slots = vec![(0u32, 0u32); capacity + OVERFLOW_SLOTS];
    let mut at = Vec::with_capacity(hashes.len());
    for (ordinal, hash) in hashes.iter().enumerate() {
        let home = home_of(*hash, capacity);
        let free =
            (home..home + MAX_PROBE).find(|s| slots.get(*s).is_some_and(|slot| slot.0 == 0))?;
        let mark = u32::try_from(ordinal).ok()?.checked_add(1)?;
        *slots.get_mut(free)? = (mark, tag_of(*hash));
        at.push((home, free));
    }
    for (hash, (home, mine)) in hashes.iter().zip(&at) {
        let tag = tag_of(*hash);
        let repeated = (*home..*home + MAX_PROBE).filter(|s| s != mine).any(|s| {
            slots
                .get(s)
                .is_some_and(|slot| slot.0 != 0 && slot.1 == tag)
        });
        if repeated {
            return None;
        }
    }
    Some(slots)
}

// --------------------------------------------------------- building images --

/// One record's place in a generation being built.
struct Planned {
    key: Key,
    run: u32,
    facts: Facts,
    details: Vec<Detail>,
}

/// A generation's bytes, built once, with its generation stamped per try.
struct Built {
    bytes: Vec<u8>,
    header: ImageHeader,
    worst: Code,
    flags: u64,
    mismatches: u32,
    capped: bool,
}

/// What a run writes for one day.
#[derive(Debug, Clone)]
pub struct Generation {
    /// The day.
    pub day: Day,
    /// The run (from the runs ledger); at most `u32::MAX`.
    pub run: u64,
    /// When it is written (unix seconds); excluded from byte identity.
    pub written_at: u64,
    /// The verifier version.
    pub verifier: u16,
    /// The pass.
    pub mode: Mode,
    /// The day's rule set (0 = not applicable).
    pub rule_set: u8,
    /// The origin digest prefix.
    pub origin: [u8; 16],
    /// The day block; `worst` is the day's own findings (see [`DayBlock`]).
    pub day_block: DayBlock,
    /// The day's own details (ordinal 0).
    pub day_details: Vec<Detail>,
    /// The keys this run verified.
    pub entries: Vec<Entry>,
    /// Whether the keys this run did not verify are carried from the
    /// committed generation.
    pub carry: Carry,
}

/// Whether a generation carries the keys its run did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carry {
    /// The entries are the whole day: nothing is carried.
    WholeDay,
    /// The entries are some keys of the day (a Zerodha window, a filtered
    /// run): every other key of the committed generation is copied whole,
    /// keeping its own run.
    Others,
}

/// What a commit published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// The new generation.
    pub generation: u32,
    /// The generation it replaced in the year index (0 = none).
    pub previous: u32,
    /// Its records.
    pub records: u32,
    /// Records carried unchanged.
    pub carried: u32,
    /// Generations stepped past because a crash left them linked.
    pub stepped_past: u32,
    /// Temporaries a dead writer of the same process number and run left at
    /// this writer's names, cleared before it wrote ([`crate::staging`]).
    /// The caller's "day verified" event carries it.
    pub staging_cleared: u32,
    /// The day's worst code.
    pub worst: Code,
    /// Whether the detail section reached [`DETAIL_CAP`].
    pub details_capped: bool,
    /// The image path.
    pub path: PathBuf,
    /// The year index's new commit generation.
    pub commit: u64,
}

/// Refuses an unknown flag or an unset or oversized detail.
fn check_entry(flags: u64, details: &[Detail]) -> Result<(), VerdictError> {
    if flags & !flags::KNOWN != 0 {
        return Err(VerdictError::FlagUnknown {
            bits: flags & !flags::KNOWN,
        });
    }
    for detail in details {
        if detail.kind == 0 || detail.payload.len() > usize::from(u16::MAX) {
            return Err(VerdictError::DetailInvalid {
                kind: detail.kind,
                len: detail.payload.len(),
            });
        }
    }
    Ok(())
}

/// Appends one detail entry's bytes.
fn push_detail(out: &mut Vec<u8>, owner: u32, detail: &Detail) {
    out.extend_from_slice(&owner.to_le_bytes());
    out.extend_from_slice(&detail.kind.to_le_bytes());
    let len = u16::try_from(detail.payload.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&detail.payload);
}

/// The bytes a list of details takes in the section.
fn details_len(details: &[Detail]) -> usize {
    details
        .iter()
        .map(|d| DETAIL_HEAD_LEN + d.payload.len())
        .sum()
}

/// The detail section of sorted records: the day's entries, then each
/// record's, all or nothing per owner. Once one owner does not fit under
/// [`DETAIL_CAP`], it and every later owner keep their counts only and carry
/// [`flags::DETAILS_CAPPED`].
struct Section {
    bytes: Vec<u8>,
    entries: u32,
    capped: bool,
    /// Per record: its first entry's offset plus one, or 0.
    first: Vec<u32>,
}

fn section(day_details: &[Detail], planned: &mut [Planned]) -> Section {
    let mut out = Section {
        bytes: Vec::new(),
        entries: 0,
        capped: false,
        first: vec![0u32; planned.len()],
    };
    if details_len(day_details) <= DETAIL_CAP {
        for detail in day_details {
            push_detail(&mut out.bytes, 0, detail);
            out.entries = out.entries.saturating_add(1);
        }
    } else {
        out.capped = true;
    }
    for (ordinal, plan) in planned.iter_mut().enumerate() {
        if plan.details.is_empty() {
            continue;
        }
        if out.capped || out.bytes.len() + details_len(&plan.details) > DETAIL_CAP {
            out.capped = true;
            plan.facts.flags |= flags::DETAILS_CAPPED;
            continue;
        }
        if let Some(slot) = out.first.get_mut(ordinal) {
            *slot = u32::try_from(out.bytes.len() + 1).unwrap_or(NONE);
        }
        let owner = u32::try_from(ordinal + 1).unwrap_or(NONE);
        for detail in &plan.details {
            push_detail(&mut out.bytes, owner, detail);
            out.entries = out.entries.saturating_add(1);
        }
    }
    out
}

/// The per-code counts, the worst code, the mismatches and the OR of every
/// record's flags with the day's.
struct Tally {
    counts: [u32; CODE_SLOTS],
    worst: Code,
    mismatches: u32,
    flags: u64,
}

fn tally(day: &DayBlock, day_flags: u64, planned: &[Planned]) -> Tally {
    let mut out = Tally {
        counts: [0u32; CODE_SLOTS],
        worst: if day.state.is_partial() {
            day.worst.worse(Code::DayPartial)
        } else {
            day.worst
        },
        mismatches: 0,
        flags: day_flags,
    };
    for plan in planned {
        let code = plan.facts.code;
        if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
            *count = count.saturating_add(1);
        }
        out.worst = out.worst.worse(code);
        if code.is_mismatch() {
            out.mismatches = out.mismatches.saturating_add(1);
        }
        out.flags |= plan.facts.flags;
    }
    out
}

/// Builds a generation's bytes, its generation field 0 until [`stamp`].
fn build(
    series: &Series<'_>,
    generation: &Generation,
    mut planned: Vec<Planned>,
    previous: u32,
    carried: u32,
) -> Result<Built, VerdictError> {
    series.feed_byte()?;
    let unit = unit_of(series.timeframe);
    // THE ONE SORT (gate 11's ALLOW_SORT, L-IA-4): records by (code, key
    // text, hash), so one code's records are one slice and the bytes do not
    // depend on the order the entries arrived in. O(n log n) per image; a
    // lookup never sorts.
    planned.sort_unstable_by(|a, b| {
        (a.facts.code, a.key.text.as_bytes(), a.key.hash).cmp(&(
            b.facts.code,
            b.key.text.as_bytes(),
            b.key.hash,
        ))
    });
    let n = planned.len();
    let hashes: Vec<u64> = planned.iter().map(|p| p.key.hash).collect();
    let (capacity, slots) = table(&hashes)?;
    let details = section(&generation.day_details, &mut planned);
    let day_flags = if details.capped {
        generation.day_block.flags | flags::DETAILS_CAPPED
    } else {
        generation.day_block.flags
    };
    let sums = tally(&generation.day_block, day_flags, &planned);

    let records_at = TABLE_OFFSET + SLOT_LEN * (capacity + OVERFLOW_SLOTS);
    let records_end = records_at + RECORD_STRIDE * n;
    let at = Path::new("(being built)");
    let header = ImageHeader {
        feed: series.feed,
        exchange: series.exchange,
        segment: series.segment,
        timeframe: series.timeframe,
        unit,
        epoch_day: epoch_day(generation.day)?,
        generation: 0,
        records: u32_of(n, at, "records")?,
        capacity: u32_of(capacity, at, "capacity")?,
        counts: sums.counts,
        run: generation.run,
        written_at: generation.written_at,
        verifier: generation.verifier,
        mode: generation.mode,
        rule_set: generation.rule_set,
        previous,
        carried,
        origin: generation.origin,
        detail_offset: if details.bytes.is_empty() {
            0
        } else {
            u32_of(records_end, at, "detail offset")?
        },
        detail_bytes: u32_of(details.bytes.len(), at, "detail bytes")?,
        detail_entries: details.entries,
    };
    let day_block = DayBlock {
        worst: sums.worst,
        flags: day_flags,
        ..generation.day_block
    };

    let mut bytes = vec![0u8; records_end + details.bytes.len() + TRAILER_LEN];
    put(&mut bytes, HEADER_LEN, &encode_day(&day_block));
    for (i, (mark, tag)) in slots.iter().enumerate() {
        let slot = TABLE_OFFSET + SLOT_LEN * i;
        put(&mut bytes, slot, &mark.to_le_bytes());
        put(&mut bytes, slot + 4, &tag.to_le_bytes());
    }
    for (ordinal, plan) in planned.iter().enumerate() {
        let first = details.first.get(ordinal).copied().unwrap_or(0);
        let record = encode_record(&plan.key, plan.run, &plan.facts, first);
        put(&mut bytes, records_at + RECORD_STRIDE * ordinal, &record);
    }
    put(&mut bytes, records_end, &details.bytes);
    Ok(Built {
        bytes,
        header,
        worst: sums.worst,
        flags: sums.flags,
        mismatches: sums.mismatches,
        capped: details.capped,
    })
}

/// Stamps a generation into built bytes: the header (and its CRC) and the
/// BLAKE3 trailer, the only bytes that depend on it.
fn stamp(built: &mut Built, generation: u32) {
    built.header.generation = generation;
    let header = encode_header(&built.header);
    put(&mut built.bytes, 0, &header);
    let end = built.bytes.len() - TRAILER_LEN;
    let digest = blake3::hash(built.bytes.get(..end).unwrap_or_default());
    put(&mut built.bytes, end, &digest);
}

// ------------------------------------------------------- reading an image --

/// A whole image, decoded and checked.
#[derive(Debug, Clone)]
pub struct Image {
    /// The header.
    pub header: ImageHeader,
    /// The day block.
    pub day: DayBlock,
    /// Every record, in ordinal order, with its details.
    pub records: Vec<(Record, Vec<Detail>)>,
    /// The day's own details.
    pub day_details: Vec<Detail>,
}

/// Decodes and checks a whole image: the BLAKE3 trailer first (so one byte
/// changed anywhere is refused [`VerdictError::TrailerMismatch`]), then the
/// header, the day block, every record's CRC-32C, the (code, key) order, the
/// per-code counts, that the table finds every record where a lookup would,
/// and the detail section's shape. O(image bytes).
fn parse(bytes: &[u8], path: &Path) -> Result<Image, VerdictError> {
    if bytes.len() < TABLE_OFFSET + TRAILER_LEN {
        return Err(corrupt(path, "shorter than a header and a day block"));
    }
    let end = bytes.len() - TRAILER_LEN;
    if blake3::hash(bytes.get(..end).unwrap_or_default()) != take::<32>(bytes, end) {
        return Err(VerdictError::TrailerMismatch {
            path: path.to_path_buf(),
        });
    }
    let header = decode_header(bytes, path)?;
    let day = decode_day(
        bytes.get(HEADER_LEN..TABLE_OFFSET).unwrap_or_default(),
        path,
    )?;
    let geo = geometry(&header, path)?;
    if bytes.len() != geo.len {
        return Err(corrupt(
            path,
            format!("{} bytes where the header implies {}", bytes.len(), geo.len),
        ));
    }
    let n = usize_of(u64::from(header.records));
    let mut records = Vec::with_capacity(n);
    for ordinal in 0..n {
        let at = geo.records_at + RECORD_STRIDE * ordinal;
        let raw = bytes.get(at..at + RECORD_STRIDE).unwrap_or_default();
        records.push(decode_record(raw, path, u32_of(ordinal, path, "ordinal")?)?);
    }
    // Order and counts.
    let mut counts = [0u32; CODE_SLOTS];
    for (i, record) in records.iter().enumerate() {
        if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
            *c = c.saturating_add(1);
        }
        if let Some(next) = records.get(i + 1) {
            let a = (
                record.facts.code,
                record.key.text.as_bytes(),
                record.key.hash,
            );
            let b = (next.facts.code, next.key.text.as_bytes(), next.key.hash);
            if a >= b {
                return Err(corrupt(
                    path,
                    format!("records {i} and {} are out of (code, key) order", i + 1),
                ));
            }
        }
    }
    if counts != header.counts {
        return Err(corrupt(path, "the per-code counts differ from the records"));
    }
    // The table finds every record where a lookup would, and holds nothing else.
    let mut marked = 0usize;
    for i in 0..geo.capacity + OVERFLOW_SLOTS {
        let at = TABLE_OFFSET + SLOT_LEN * i;
        let mark = u32_at(bytes, at);
        if mark == 0 {
            if u32_at(bytes, at + 4) != 0 {
                return Err(corrupt(path, format!("empty slot {i} carries a tag")));
            }
            continue;
        }
        marked += 1;
        let ordinal = usize_of(u64::from(mark - 1));
        let record = records.get(ordinal).ok_or_else(|| {
            corrupt(
                path,
                format!("slot {i} names record {ordinal}, past the records"),
            )
        })?;
        if u32_at(bytes, at + 4) != record.key.tag() {
            return Err(corrupt(
                path,
                format!("slot {i}'s tag is not record {ordinal}'s"),
            ));
        }
    }
    if marked != n {
        return Err(corrupt(
            path,
            format!("{marked} occupied slots for {n} records"),
        ));
    }
    for (ordinal, record) in records.iter().enumerate() {
        match probe(bytes, geo.capacity, &record.key) {
            Some(found) if found == ordinal => {}
            _ => {
                return Err(corrupt(
                    path,
                    format!("record {ordinal} is not where its probe window finds it"),
                ));
            }
        }
    }
    // The detail section.
    let owned = parse_details(bytes, &header, &records, path)?;
    Ok(Image {
        header,
        day,
        records: records.into_iter().zip(owned.per).collect(),
        day_details: owned.day,
    })
}

/// The ordinal a probe window finds for `key` in a whole image's bytes.
fn probe(bytes: &[u8], capacity: usize, key: &Key) -> Option<usize> {
    let home = home_of(key.hash, capacity);
    let window =
        bytes.get(TABLE_OFFSET + SLOT_LEN * home..TABLE_OFFSET + SLOT_LEN * (home + MAX_PROBE))?;
    first_tagged(window, key.tag())
}

/// The ordinal of the first slot in a probe window whose tag is `tag`.
fn first_tagged(window: &[u8], tag: u32) -> Option<usize> {
    window.chunks_exact(SLOT_LEN).find_map(|slot| {
        let mark = u32_at(slot, 0);
        (mark != 0 && u32_at(slot, 4) == tag).then(|| usize_of(u64::from(mark - 1)))
    })
}

/// The day's details and each record's, checked: entries sorted by owner, a
/// record's `detail_at` pointing at its first entry, and the counts the
/// header states.
fn parse_details(
    bytes: &[u8],
    header: &ImageHeader,
    records: &[Record],
    path: &Path,
) -> Result<Owned, VerdictError> {
    let mut day = Vec::new();
    let mut per: Vec<Vec<Detail>> = vec![Vec::new(); records.len()];
    let start = usize_of(u64::from(header.detail_offset));
    let len = usize_of(u64::from(header.detail_bytes));
    let section = if len == 0 {
        &[][..]
    } else {
        bytes.get(start..start + len).unwrap_or_default()
    };
    let mut at = 0usize;
    let mut entries = 0u32;
    let mut last_owner = 0u32;
    let mut firsts: HashMap<u32, usize> = HashMap::with_capacity(records.len().min(4096) + 1);
    while at < section.len() {
        let owner = u32_at(section, at);
        let kind = u16_at(section, at + 4);
        let size = usize::from(u16_at(section, at + 6));
        let payload = section
            .get(at + DETAIL_HEAD_LEN..at + DETAIL_HEAD_LEN + size)
            .ok_or_else(|| corrupt(path, format!("detail entry at {at} runs past the section")))?;
        if owner < last_owner || kind == 0 {
            return Err(corrupt(
                path,
                format!("detail entry at {at} is out of order or unset"),
            ));
        }
        let detail = Detail {
            kind,
            payload: payload.to_vec(),
        };
        if owner == 0 {
            day.push(detail);
        } else {
            let slot = per
                .get_mut(usize_of(u64::from(owner - 1)))
                .ok_or_else(|| corrupt(path, format!("detail entry at {at} names no record")))?;
            if owner != last_owner {
                firsts.insert(owner - 1, at);
            }
            slot.push(detail);
        }
        last_owner = owner;
        at += DETAIL_HEAD_LEN + size;
        entries = entries.saturating_add(1);
    }
    if entries != header.detail_entries {
        return Err(corrupt(
            path,
            "the detail entries differ from the header's count",
        ));
    }
    for (ordinal, record) in records.iter().enumerate() {
        let first = u32::try_from(ordinal)
            .ok()
            .and_then(|o| firsts.get(&o))
            .map_or(0, |at| u32::try_from(*at + 1).unwrap_or(NONE));
        if first != record.detail_at {
            return Err(corrupt(
                path,
                format!("record {ordinal}'s detail offset is not its first entry"),
            ));
        }
    }
    Ok(Owned { day, per })
}

/// The day's details and each record's, in ordinal order.
struct Owned {
    day: Vec<Detail>,
    per: Vec<Vec<Detail>>,
}

/// Reads and checks a whole image file. O(image bytes): [`check_trailer`],
/// the carry of a partial generation, and the tests read through it; a
/// lookup never does.
///
/// # Errors
///
/// [`VerdictError::Io`] when the file cannot be read, and every refusal
/// [`check_trailer`] names.
pub fn read_image(path: &Path) -> Result<Image, VerdictError> {
    let file = open_read(path)?;
    let len = file
        .metadata()
        .map_err(|why| io_error(path, "stat", why))?
        .len();
    let mut bytes = vec![0u8; usize_of(len)];
    file.read_exact_at(&mut bytes, 0)
        .map_err(|why| io_error(path, "read", why))?;
    parse(&bytes, path)
}

/// What [`check_trailer`] found in one whole image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checked {
    /// Its generation.
    pub generation: u32,
    /// Its records.
    pub records: u32,
    /// Its detail entries.
    pub details: u32,
}

/// The scrub of one day image, and the one reader of its BLAKE3 trailer
/// (IA-VD-14; WP-81's store scrub calls it over `verify/`): every byte read
/// once, the trailer checked first, so one byte changed anywhere is refused
/// [`VerdictError::TrailerMismatch`]; then every CRC-32C, the order, the
/// counts, the table and the detail section. A lookup never reads the
/// trailer. O(image bytes).
///
/// # Errors
///
/// Every check names its refusal: [`VerdictError::TrailerMismatch`],
/// [`VerdictError::ImageCorrupt`], [`VerdictError::RecordCorrupt`],
/// [`VerdictError::Unknown`], [`VerdictError::Symlinked`],
/// [`VerdictError::Io`].
pub fn check_trailer(path: &Path) -> Result<Checked, VerdictError> {
    let image = read_image(path)?;
    Ok(Checked {
        generation: image.header.generation,
        records: image.header.records,
        details: image.header.detail_entries,
    })
}

// --------------------------------------------------------- the year index --

/// One day's entry of the year index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DayEntry {
    /// The current generation (0 = never verified).
    pub generation: u32,
    /// The day's worst code (0 = unset).
    pub worst: u8,
    /// The day state's byte.
    pub state: u8,
    /// The verifier version that wrote it.
    pub verifier: u16,
    /// Records.
    pub records: u32,
    /// Mismatches.
    pub mismatches: u32,
    /// The OR of the day's record and day flags.
    pub flags: u64,
}

impl DayEntry {
    /// The worst code, or `None` for a day never verified.
    #[must_use]
    pub const fn worst_code(&self) -> Option<Code> {
        Code::from_code(self.worst)
    }
}

/// A year index as it reads back: the valid slot with the highest commit
/// generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearIndex {
    /// The year.
    pub year: u16,
    /// The commit generation.
    pub commit: u64,
    /// Which slot it came from (0 or 1).
    pub slot: u8,
    /// One entry per day of the year, 1 January first.
    pub days: Vec<DayEntry>,
}

impl YearIndex {
    /// A day's entry.
    #[must_use]
    pub fn entry(&self, day: Day) -> DayEntry {
        self.days.get(day_of_year(day)).copied().unwrap_or_default()
    }
}

/// The 8,812 bytes of one slot.
fn encode_slot(year: u16, commit: u64, days: &[DayEntry]) -> Vec<u8> {
    let mut out = vec![0u8; INDEX_SLOT_LEN];
    put(&mut out, 0, &INDEX_MAGIC);
    put(&mut out, 8, &INDEX_VERSION.to_le_bytes());
    put(&mut out, 12, &year.to_le_bytes());
    put(&mut out, 16, &commit.to_le_bytes());
    for (i, entry) in days.iter().take(YEAR_DAYS).enumerate() {
        let at = INDEX_HEAD_LEN + DAY_ENTRY_LEN * i;
        put(&mut out, at, &entry.generation.to_le_bytes());
        put(&mut out, at + 4, &[entry.worst, entry.state]);
        put(&mut out, at + 6, &entry.verifier.to_le_bytes());
        put(&mut out, at + 8, &entry.records.to_le_bytes());
        put(&mut out, at + 12, &entry.mismatches.to_le_bytes());
        put(&mut out, at + 16, &entry.flags.to_le_bytes());
    }
    let crc = crc_to(&out, INDEX_SLOT_LEN - 4);
    put(&mut out, INDEX_SLOT_LEN - 4, &crc.to_le_bytes());
    out
}

/// One slot, or `None` when it is torn, foreign or of another year.
fn decode_slot(bytes: &[u8], year: u16) -> Option<(u64, Vec<DayEntry>)> {
    let valid = bytes.len() == INDEX_SLOT_LEN
        && take::<8>(bytes, 0) == INDEX_MAGIC
        && u16_at(bytes, 8) == INDEX_VERSION
        && u16_at(bytes, 12) == year
        && crc_to(bytes, INDEX_SLOT_LEN - 4) == u32_at(bytes, INDEX_SLOT_LEN - 4);
    if !valid {
        return None;
    }
    let days = (0..YEAR_DAYS)
        .map(|i| {
            let at = INDEX_HEAD_LEN + DAY_ENTRY_LEN * i;
            DayEntry {
                generation: u32_at(bytes, at),
                worst: u8_at(bytes, at + 4),
                state: u8_at(bytes, at + 5),
                verifier: u16_at(bytes, at + 6),
                records: u32_at(bytes, at + 8),
                mismatches: u32_at(bytes, at + 12),
                flags: u64_at(bytes, at + 16),
            }
        })
        .collect();
    Some((u64_at(bytes, 16), days))
}

/// Both slots read (2 positional reads), the valid one with the highest
/// commit generation kept; slot 0 on a tie.
fn read_year(
    file: &File,
    path: &Path,
    year: u16,
    cost: &mut Cost,
) -> Result<YearIndex, VerdictError> {
    let mut best: Option<YearIndex> = None;
    for slot in 0..2u8 {
        let mut bytes = vec![0u8; INDEX_SLOT_LEN];
        file.read_exact_at(&mut bytes, u64::from(slot) * SLOT_STRIDE_U64)
            .map_err(|why| io_error(path, "read", why))?;
        cost.reads += 1;
        if let Some((commit, days)) = decode_slot(&bytes, year)
            && best.as_ref().is_none_or(|b| commit > b.commit)
        {
            best = Some(YearIndex {
                year,
                commit,
                slot,
                days,
            });
        }
    }
    best.ok_or_else(|| VerdictError::IndexUnreadable {
        path: path.to_path_buf(),
    })
}

// -------------------------------------------------------------- the files --

/// Opens for reading, never through a final symbolic link.
fn open_read(path: &Path) -> Result<File, VerdictError> {
    let mut options = File::options();
    options.read(true);
    no_follow(&mut options);
    options.open(path).map_err(|why| opened(path, why))
}

/// Opens an existing file for reading and writing, never creating it and
/// never through a final link. A year index is only ever born by
/// [`genesis`], so an open that created one would be the half file D-4865
/// refuses.
fn open_existing(path: &Path) -> Result<File, VerdictError> {
    let mut options = File::options();
    options.read(true).write(true);
    no_follow(&mut options);
    options.open(path).map_err(|why| opened(path, why))
}

/// Opens the commit lock file for writing, creating it, never through a
/// final link and NEVER truncating: the lock file is shared, holds no byte
/// and is never written, so whatever is at its name is not changed by this
/// open. A writer's own temporary is not opened here: it is born by
/// [`staging::create`], which opens nothing that already exists (RA-290).
fn open_lock(path: &Path) -> Result<File, VerdictError> {
    let mut options = File::options();
    options.read(true).write(true).create(true).truncate(false);
    no_follow(&mut options);
    options.open(path).map_err(|why| opened(path, why))
}

/// Takes the commit lock within `wait`, in a counted loop of
/// [`LOCK_STEP`]s: never a blocking wait, so a holder that hangs is a named
/// refusal and not a writer that never returns (D-4924).
fn lock_within<'f>(
    file: &'f File,
    path: &Path,
    wait: Duration,
) -> Result<Flock<&'f File, PathBuf>, VerdictError> {
    let steps = u32::try_from(wait.as_millis() / LOCK_STEP.as_millis().max(1)).unwrap_or(u32::MAX);
    let mut step = 0u32;
    loop {
        match Flock::try_lock(file, path.to_path_buf()) {
            Ok(lock) => return Ok(lock),
            Err(TryLockError::WouldBlock) if step < steps => {
                step += 1;
                std::thread::sleep(LOCK_STEP);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(VerdictError::LockStalled {
                    path: path.to_path_buf(),
                    waited_ms: u64::try_from(wait.as_millis()).unwrap_or(u64::MAX),
                    tries: step.saturating_add(1),
                });
            }
            Err(TryLockError::Error(why)) => return Err(io_error(path, "lock", why)),
        }
    }
}

/// `O_NOFOLLOW` where `crate::open_flags` has a value for it.
fn no_follow(options: &mut fs::OpenOptions) {
    #[cfg(any(
        target_os = "macos",
        all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(crate::open_flags::O_NOFOLLOW);
    }
    #[cfg(not(any(
        target_os = "macos",
        all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    let _ = options;
}

/// An open's failure, named: a final link refused under `O_NOFOLLOW`.
fn opened(path: &Path, why: io::Error) -> VerdictError {
    #[cfg(any(
        target_os = "macos",
        all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
        return VerdictError::Symlinked {
            path: path.to_path_buf(),
        };
    }
    io_error(path, "open", why)
}

/// Makes a directory's entries durable.
fn sync_dir(dir: &Path) -> Result<(), VerdictError> {
    File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| io_error(dir, "sync directory", why))
}

/// Writes all of `bytes` to a NEW file at `path` and makes them durable.
/// The file is born by [`staging::create`]: a name that already exists is
/// never opened, so a hard link planted at a temporary's name is refused
/// `staging_linked` with the file behind it untouched (RA-290). Returns
/// whether a dead writer's leftover of the same name was cleared first.
fn write_durable(path: &Path, bytes: &[u8]) -> Result<bool, VerdictError> {
    let staged = staging::create(path).map_err(|why| match why {
        StagingError::Io {
            path,
            action,
            source,
        } => VerdictError::Io {
            path,
            action,
            source,
        },
        other => VerdictError::Staging(other),
    })?;
    staged
        .file
        .write_all_at(bytes, 0)
        .map_err(|why| io_error(path, "write", why))?;
    staged
        .file
        .sync_all()
        .map_err(|why| io_error(path, "sync", why))?;
    Ok(staged.cleared.is_some())
}

/// The writer-unique temporary beside `path`: `<name>.<pid>-<run>.tmp`.
fn tmp_of(path: &Path, run: u64) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}-{run}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Removes a temporary this writer made; one already gone is fine.
fn remove_tmp(path: &Path) -> Result<(), VerdictError> {
    match fs::remove_file(path) {
        Err(why) if why.kind() != ErrorKind::NotFound => Err(io_error(path, "remove", why)),
        _ => Ok(()),
    }
}

/// Builds a year index beside its final name and links it into place: born
/// whole (D-4865), both slots written to a writer-unique temporary, made
/// durable, and linked with `hard_link`, which never replaces a file. The
/// loser of a creation race reads the winner's.
fn genesis(series: &Series<'_>, year: u16, run: u64) -> Result<(File, bool), VerdictError> {
    let path = series.year_index(year);
    let tmp = tmp_of(&path, run);
    let empty = vec![DayEntry::default(); YEAR_DAYS];
    let slot = encode_slot(year, 0, &empty);
    let mut bytes = vec![0u8; INDEX_LEN];
    put(&mut bytes, 0, &slot);
    put(&mut bytes, INDEX_SLOT_STRIDE, &slot);
    let cleared = write_durable(&tmp, &bytes)?;
    #[cfg(test)]
    tests::crash("index tmp written")?;
    match fs::hard_link(&tmp, &path) {
        Ok(()) => {}
        Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
        Err(why) => {
            remove_tmp(&tmp)?;
            return Err(io_error(&path, "link", why));
        }
    }
    remove_tmp(&tmp)?;
    sync_dir(&series.dir())?;
    Ok((open_existing(&path)?, cleared))
}

/// The year index's length, refused below [`INDEX_LEN`].
fn index_len(file: &File, path: &Path) -> Result<u64, VerdictError> {
    let len = file
        .metadata()
        .map_err(|why| io_error(path, "stat", why))?
        .len();
    if len < INDEX_LEN_U64 {
        return Err(VerdictError::IndexShort {
            path: path.to_path_buf(),
            len,
        });
    }
    Ok(len)
}

/// Creates the series' year directory, making each new directory entry
/// durable.
fn make_dirs(series: &Series<'_>, year: u16) -> Result<PathBuf, VerdictError> {
    let dir = series.dir().join(format!("{year:04}"));
    if dir.is_dir() {
        return Ok(dir);
    }
    fs::create_dir_all(&dir).map_err(|why| io_error(&dir, "create directory", why))?;
    let mut at = dir.as_path();
    while let Some(parent) = at.parent() {
        sync_dir(parent)?;
        if parent == series.root {
            break;
        }
        at = parent;
    }
    Ok(dir)
}

// ------------------------------------------------------------ the commit --

/// The run as a record's `u32`, the series and day checked, and every
/// entry's flags, details and key: the refusals made before any lock.
fn validate(series: &Series<'_>, generation: &Generation) -> Result<u32, VerdictError> {
    let run = u32::try_from(generation.run).map_err(|_| VerdictError::RunIdOverflow {
        run: generation.run,
    })?;
    series.feed_byte()?;
    epoch_day(generation.day)?;
    check_entry(generation.day_block.flags, &generation.day_details)?;
    for entry in &generation.entries {
        check_entry(entry.facts.flags, &entry.details)?;
    }
    Ok(run)
}

/// The records of the new generation: this run's entries (each stamped with
/// the run; a cut stem carrying [`flags::NAME_CUT`] and its whole name in
/// detail [`DETAIL_NAME`]) and, for [`Carry::Others`], every committed record
/// whose key this run did not verify, copied whole with its own run.
fn plan(
    series: &Series<'_>,
    generation: &Generation,
    run: u32,
    committed: u32,
) -> Result<(Vec<Planned>, u32), VerdictError> {
    let mut seen: HashSet<(u64, String)> = HashSet::with_capacity(generation.entries.len() + 1);
    let mut planned: Vec<Planned> = Vec::with_capacity(generation.entries.len());
    for entry in &generation.entries {
        if !seen.insert((entry.key.hash, entry.key.text.clone())) {
            return Err(VerdictError::DuplicateKey {
                key: entry.key.to_string(),
            });
        }
        let mut facts = entry.facts;
        let mut details = entry.details.clone();
        if let Some(whole) = entry.key.whole() {
            facts.flags |= flags::NAME_CUT;
            details.insert(
                0,
                Detail {
                    kind: DETAIL_NAME,
                    payload: whole.as_bytes().to_vec(),
                },
            );
            check_entry(facts.flags, &details)?;
        }
        planned.push(Planned {
            key: entry.key.clone(),
            run,
            facts,
            details,
        });
    }
    let mut carried = 0u32;
    if generation.carry == Carry::Others && committed > 0 {
        let path = series.image_path(generation.day, committed);
        let image = match read_image(&path) {
            Err(VerdictError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => {
                return Err(VerdictError::ImageMissing { path });
            }
            other => other?,
        };
        for (record, details) in image.records {
            // A key this run verified is already in the set, so the insert
            // refuses it and the run's own verdict stands.
            if !seen.insert((record.key.hash, record.key.text.clone())) {
                continue;
            }
            carried = carried.saturating_add(1);
            planned.push(Planned {
                key: record.key,
                run: record.run,
                facts: record.facts,
                details,
            });
        }
    }
    Ok((planned, carried))
}

/// What [`publish`] linked.
struct Published {
    /// The generation.
    generation: u32,
    /// Its image.
    path: PathBuf,
    /// Generations a crash had left linked.
    stepped_past: u32,
    /// Temporaries of a dead writer cleared on the way.
    cleared: u32,
}

/// Publishes built bytes as the first free generation after `committed`:
/// written to a writer-unique temporary, made durable, linked with
/// `hard_link` (which fails rather than replace a file), the temporary
/// removed, and the directory made durable.
fn publish(
    series: &Series<'_>,
    generation: &Generation,
    built: &mut Built,
    committed: u32,
) -> Result<Published, VerdictError> {
    let mut stepped_past = 0u32;
    let mut cleared = 0u32;
    for attempt in 1..=MAX_LINK_TRIES {
        let g = committed.checked_add(attempt).ok_or_else(|| {
            corrupt(
                &series.year_index(generation.day.year()),
                "the generation counter is exhausted",
            )
        })?;
        stamp(built, g);
        let path = series.image_path(generation.day, g);
        let tmp = tmp_of(&path, generation.run);
        cleared += u32::from(write_durable(&tmp, &built.bytes)?);
        #[cfg(test)]
        tests::crash("image tmp written")?;
        match fs::hard_link(&tmp, &path) {
            Ok(()) => {
                remove_tmp(&tmp)?;
                sync_dir(path.parent().unwrap_or(series.root))?;
                return Ok(Published {
                    generation: g,
                    path,
                    stepped_past,
                    cleared,
                });
            }
            Err(why) if why.kind() == ErrorKind::AlreadyExists => {
                remove_tmp(&tmp)?;
                stepped_past += 1;
            }
            Err(why) => {
                remove_tmp(&tmp)?;
                return Err(io_error(&path, "link", why));
            }
        }
    }
    Err(VerdictError::GenerationCollision {
        path: series.image_path(generation.day, committed.saturating_add(MAX_LINK_TRIES)),
    })
}

/// Publishes one generation of one day, under the commit lock held across
/// the whole write (D-4865): read the year index and the day's committed
/// generation `g`; read image `g` for the carry; build the image; publish it
/// as `g + 1` by `hard_link`, stepping past any generation a crash left
/// linked and uncommitted (at most [`MAX_LINK_TRIES`]); then write the year
/// index's next slot and make it durable. Two writers on one day never both
/// carry from `g`, and no linked image is ever overwritten.
///
/// Every refusal is made before anything is published. The inputs are
/// checked before any file is touched; a whole-day generation's records are
/// its entries alone, so it is also planned and built then (`duplicate_key`,
/// and `table_too_large` when it cannot be placed, L-IA-33): a refused
/// whole-day commit writes nothing, no directory, lock, index or image. A
/// carrying generation can only be planned and built under the lock; its
/// refusal leaves the series' directories and the lock file, never a year
/// index or an image (a missing `.vdi` is born only once the image is
/// built).
///
/// Cost: O(n log n) for the one sort of n records, and 3 `sync_all`s
/// (L-IA-4); not a lookup's bound, and `crates/store/benches/ratio.rs`
/// (C-VD-04) times it.
///
/// # Errors
///
/// Every input refusal ([`VerdictError::DuplicateKey`],
/// [`VerdictError::RunIdOverflow`], [`VerdictError::FlagUnknown`],
/// [`VerdictError::DetailInvalid`], [`VerdictError::TableTooLarge`],
/// [`VerdictError::DayOutOfRange`], [`VerdictError::FeedUnknown`]),
/// [`VerdictError::GenerationCollision`], a refusal reading the year index
/// or the committed image for the carry, and the host's.
pub fn commit(series: &Series<'_>, generation: &Generation) -> Result<Committed, VerdictError> {
    commit_within(series, generation, LOCK_WAIT)
}

/// [`commit`] with the caller's own wait for the commit lock: past `wait`
/// the commit is refused `lock_stalled` with nothing published (D-4924).
/// `Duration::ZERO` tries the lock once.
///
/// # Errors
///
/// As [`commit`], and [`VerdictError::LockStalled`].
pub fn commit_within(
    series: &Series<'_>,
    generation: &Generation,
    wait: Duration,
) -> Result<Committed, VerdictError> {
    let run = validate(series, generation)?;
    let early = match generation.carry {
        Carry::WholeDay => {
            let (planned, _) = plan(series, generation, run, 0)?;
            Some(build(series, generation, planned, 0, 0)?)
        }
        Carry::Others => None,
    };
    let year = generation.day.year();
    make_dirs(series, year)?;
    let lock_path = series.lock_path(year);
    let lock_file = open_lock(&lock_path)?;
    let lock = lock_within(&lock_file, &lock_path, wait)?;

    let index_path = series.year_index(year);
    let held = match open_existing(&index_path) {
        Ok(file) => Some(file),
        Err(VerdictError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => None,
        Err(other) => return Err(other),
    };
    let current = match &held {
        Some(file) => {
            index_len(file, &index_path)?;
            read_year(file, &index_path, year, &mut Cost::default())?
        }
        None => YearIndex {
            year,
            commit: 0,
            slot: 0,
            days: vec![DayEntry::default(); YEAR_DAYS],
        },
    };
    let doy = day_of_year(generation.day);
    let committed = current
        .days
        .get(doy)
        .copied()
        .unwrap_or_default()
        .generation;

    let (mut built, carried) = if let Some(built) = early {
        (built, 0)
    } else {
        let (planned, carried) = plan(series, generation, run, committed)?;
        (
            build(series, generation, planned, committed, carried)?,
            carried,
        )
    };
    built.header.previous = committed;
    let (index, born_over) = match held {
        Some(file) => (file, false),
        None => genesis(series, year, generation.run)?,
    };
    let Published {
        generation: g,
        path,
        stepped_past,
        cleared,
    } = publish(series, generation, &mut built, committed)?;
    #[cfg(test)]
    tests::crash("image linked")?;

    let mut days = current.days;
    if let Some(entry) = days.get_mut(doy) {
        *entry = DayEntry {
            generation: g,
            worst: built.worst.code(),
            state: generation.day_block.state.code(),
            verifier: generation.verifier,
            records: built.header.records,
            mismatches: built.mismatches,
            flags: built.flags,
        };
    }
    let commit = current.commit.saturating_add(1);
    index
        .write_all_at(
            &encode_slot(year, commit, &days),
            (commit % 2) * SLOT_STRIDE_U64,
        )
        .map_err(|why| io_error(&index_path, "write", why))?;
    index
        .sync_all()
        .map_err(|why| io_error(&index_path, "sync", why))?;
    lock.release()
        .map_err(|why| io_error(&lock_path, "unlock", why.why))?;
    Ok(Committed {
        generation: g,
        previous: committed,
        records: built.header.records,
        carried,
        stepped_past,
        staging_cleared: cleared.saturating_add(u32::from(born_over)),
        worst: built.worst,
        details_capped: built.capped,
        path,
        commit,
    })
}

impl Committed {
    /// Whether the day holds at least [`RECORDS_WATCH`] records: half of the
    /// most the format has been measured to place (L-IA-33). The caller
    /// raises the alert; the commit itself succeeded.
    #[must_use]
    pub const fn near_ceiling(&self) -> bool {
        self.records >= RECORDS_WATCH
    }
}

// ------------------------------------------------------------- the reader --

/// What one lookup cost: files opened and positional reads made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    /// Files opened (a cached handle opens nothing).
    pub opens: u32,
    /// Positional reads.
    pub reads: u32,
}

/// Where an answer came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// The key's own record.
    Record {
        /// Its ordinal.
        ordinal: u32,
        /// The record.
        record: Record,
    },
    /// The image holds no record for the key: the key's own answer, from the
    /// day block's state, never the day's worst code (D-4844).
    NoRecord(NoRecord),
}

/// The answer for a key a day's image holds no record for (D-4844; §6.6
/// step 4 of the design). It comes from the day block's state and source
/// fields, which the lookup's one 512-byte read already holds, and is never
/// the day block's worst code: a share absent from the source on a day
/// another share is `value_mismatch` reads [`NoRecord::NotInDay`], never that
/// mismatch (IA-VD-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoRecord {
    /// `day_partial`: the day closed stopped, `source_lost`, `disk_full`, at
    /// the census or `held_back_cap`, and this key was not reached (D-4854:
    /// a partial day is never green).
    DayPartial,
    /// `source_lacks_day`: the source does not hold the day.
    SourceLacksDay,
    /// `owed (not_imported)`: the day was not imported (D-4879).
    NotImported,
    /// `not_compared (verify_off)`: the day was imported with verification
    /// off.
    VerifyOff,
    /// `no_record`: a complete day whose verification holds no record for
    /// the key (the source listed no file for it that day, or the key is
    /// outside the scope verified). Shown as "not in this day's
    /// verification": never green, and never another key's code.
    NotInDay,
}

impl NoRecord {
    /// The answer a day block gives a key it holds no record for, checked in
    /// this order: a partial state, a source that lacks the day, a day not
    /// imported, verification off, and otherwise a complete day.
    #[must_use]
    pub const fn of(day: &DayBlock) -> Self {
        if day.state.is_partial() {
            Self::DayPartial
        } else if matches!(day.source_held, SourceHeld::No) {
            Self::SourceLacksDay
        } else if matches!(day.state, DayState::NotImported) {
            Self::NotImported
        } else if matches!(day.state, DayState::VerifyOff) {
            Self::VerifyOff
        } else {
            Self::NotInDay
        }
    }

    /// The verdict code it shows, or `None` for `no_record`, which is a
    /// lookup answer and not a code.
    #[must_use]
    pub const fn code(self) -> Option<Code> {
        match self {
            Self::DayPartial => Some(Code::DayPartial),
            Self::SourceLacksDay => Some(Code::SourceLacksDay),
            Self::NotImported => Some(Code::Owed),
            Self::VerifyOff => Some(Code::NotCompared),
            Self::NotInDay => None,
        }
    }

    /// The word a route shows: the code's word, or `no_record`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self.code() {
            Some(code) => code.word(),
            None => "no_record",
        }
    }

    /// The reason beside the code where the design names one: `not_imported`
    /// for `owed`, `verify_off` for `not_compared`. `pull`'s `REASONS`
    /// numbers them; a no-record answer has no record to hold the number, so
    /// `store` gives the word.
    #[must_use]
    pub const fn reason(self) -> Option<&'static str> {
        match self {
            Self::NotImported => Some("not_imported"),
            Self::VerifyOff => Some("verify_off"),
            Self::DayPartial | Self::SourceLacksDay | Self::NotInDay => None,
        }
    }
}

/// One answer of [`Reader::lookup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The image's header.
    pub header: ImageHeader,
    /// The image's day block.
    pub day: DayBlock,
    /// The key's record, or the key's own answer when it has none.
    pub found: Found,
}

impl Answer {
    /// The code this answer shows: the record's own, or the no-record
    /// answer's, which is `None` for `no_record`. Never the day block's worst
    /// code for a key with no record (D-4844).
    #[must_use]
    pub const fn code(&self) -> Option<Code> {
        match &self.found {
            Found::Record { record, .. } => Some(record.facts.code),
            Found::NoRecord(none) => none.code(),
        }
    }

    /// The flags this answer carries: the record's, or the day's for a key
    /// with no record. A bit this build does not name is kept (D-4844).
    #[must_use]
    pub const fn flags(&self) -> u64 {
        match &self.found {
            Found::Record { record, .. } => record.facts.flags,
            Found::NoRecord(_) => self.day.flags,
        }
    }

    /// Whether the verdict is stale, computed now and never stored. Checked
    /// in order: the verifier (the image's verifier version is older than
    /// the running build's), a pending cross-kind input (`deferred_pending`),
    /// and the month (IA-6's probe, handed in as [`MonthNow`]: another served
    /// generation, or another first row or row count for the day). A month
    /// with no `.tix`, or one a writer holds, is `stale: unknown`, never
    /// fresh. A check the caller did not make is named in
    /// [`Staleness::unchecked`], and the answer is then not fresh. The
    /// staleness rule and its probe are IA-6's; this compares what the probe
    /// read with what the record holds (offsets 124 to 135).
    #[must_use]
    pub fn staleness(&self, now: &Now) -> Staleness {
        let mut out = Staleness::default();
        if self.header.verifier < now.verifier {
            out.stale.push(Stale::Verifier {
                image: self.header.verifier,
                running: now.verifier,
            });
        }
        if self.flags() & flags::DEFERRED_PENDING != 0 {
            out.stale.push(Stale::CrossKindPending);
        }
        match (&self.found, now.month) {
            (Found::NoRecord(_), _) | (_, MonthNow::NotChecked) => out.unchecked.push(Check::Month),
            (Found::Record { .. }, MonthNow::NoTix) => {
                out.stale.push(Stale::StoreUnknown(Unknown::NoTix));
            }
            (Found::Record { .. }, MonthNow::Busy) => {
                out.stale.push(Stale::StoreUnknown(Unknown::MonthBusy));
            }
            (
                Found::Record { record, .. },
                MonthNow::Read {
                    generation,
                    first_row,
                    rows,
                },
            ) => {
                let held = (
                    record.facts.month_generation,
                    record.facts.first_row,
                    record.facts.rows,
                );
                if held != (generation, first_row, rows) {
                    out.stale.push(Stale::StoreChanged {
                        verified: held,
                        now: (generation, first_row, rows),
                    });
                }
            }
        }
        out
    }
}

/// What the caller knows now, for [`Answer::staleness`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    /// The running build's verifier version.
    pub verifier: u16,
    /// What the instrument's month says for the day now.
    pub month: MonthNow,
}

/// The instrument's month for the day, as the caller's probe read it (IA-6:
/// `BarFile::open_existing`, which resolves the served generation, and the
/// `.tix`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonthNow {
    /// Not probed.
    NotChecked,
    /// The month has no `.tix`, so its day range has no positional index to
    /// read: `stale: unknown (no .tix)`.
    NoTix,
    /// A writer holds the month: the probe's shared lock failed at once, and
    /// a page lookup never waits on a writer or retries: `stale: unknown
    /// (month busy)`, never fresh (D-4844).
    Busy,
    /// Read.
    Read {
        /// The month generation read (0 = the original file).
        generation: u32,
        /// The day's first row.
        first_row: u32,
        /// The day's row count.
        rows: u32,
    },
}

/// Why a month's state is unknown to a staleness check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unknown {
    /// The month has no `.tix`.
    NoTix,
    /// A writer holds the month.
    MonthBusy,
}

impl Unknown {
    /// The words shown in `stale: unknown (<words>)`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::NoTix => "no .tix",
            Self::MonthBusy => "month busy",
        }
    }
}

/// One reason a verdict is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stale {
    /// An older verifier wrote it.
    Verifier {
        /// The image's verifier version.
        image: u16,
        /// The running build's.
        running: u16,
    },
    /// A cross-kind input is still pending.
    CrossKindPending,
    /// The month changed since: (generation, first row, rows) then and now.
    StoreChanged {
        /// What the record holds.
        verified: (u32, u32, u32),
        /// What the month says now.
        now: (u32, u32, u32),
    },
    /// The month could not be checked, because it has no `.tix` or a writer
    /// holds it: `stale: unknown (...)`, never fresh.
    StoreUnknown(Unknown),
}

impl Stale {
    /// The word a route shows: `verifier`, `cross_kind_pending`,
    /// `store_changed`, or `unknown`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Verifier { .. } => "verifier",
            Self::CrossKindPending => "cross_kind_pending",
            Self::StoreChanged { .. } => "store_changed",
            Self::StoreUnknown(_) => "unknown",
        }
    }
}

impl fmt::Display for Stale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoreUnknown(why) => write!(f, "stale: unknown ({})", why.word()),
            other => write!(f, "stale: {}", other.word()),
        }
    }
}

/// A check the caller did not make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// The month: not probed, or the answer has no record whose row range
    /// could be compared.
    Month,
}

/// A verdict's staleness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Staleness {
    /// Every reason that holds, in check order.
    pub stale: Vec<Stale>,
    /// Every check not made.
    pub unchecked: Vec<Check>,
}

impl Staleness {
    /// Fresh only when every check was made and none holds.
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        self.stale.is_empty() && self.unchecked.is_empty()
    }
}

/// One lookup's result: `answer` is `None` for a day never verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    /// What it cost.
    pub cost: Cost,
    /// The answer, or `None` when the day has no generation.
    pub answer: Option<Answer>,
}

/// A cached handle.
#[derive(Debug)]
struct Handle {
    file: File,
    len: u64,
    used: u64,
}

/// The reader: lookups through a bounded cache of [`HANDLE_CACHE`] handles,
/// least recently used out (D-4844). Year indexes are re-read on every
/// lookup (another process may commit), and images are immutable, so a
/// cached handle never serves stale bytes.
#[derive(Debug)]
pub struct Reader {
    handles: HashMap<PathBuf, Handle>,
    tick: u64,
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    /// A reader with an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handles: HashMap::with_capacity(HANDLE_CACHE + 1),
            tick: 0,
        }
    }

    /// Handles held.
    #[must_use]
    pub fn cached(&self) -> usize {
        self.handles.len()
    }

    /// The handle for `path`, opened and cached when absent; `None` when
    /// the file does not exist.
    fn handle(
        &mut self,
        path: &Path,
        cost: &mut Cost,
    ) -> Result<Option<(&File, u64)>, VerdictError> {
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        if !self.handles.contains_key(path) {
            let file = match open_read(path) {
                Ok(file) => file,
                Err(VerdictError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => {
                    return Ok(None);
                }
                Err(other) => return Err(other),
            };
            cost.opens += 1;
            let len = file
                .metadata()
                .map_err(|why| io_error(path, "stat", why))?
                .len();
            if self.handles.len() >= HANDLE_CACHE
                && let Some(oldest) = self
                    .handles
                    .iter()
                    .min_by_key(|(_, h)| h.used)
                    .map(|(p, _)| p.clone())
            {
                self.handles.remove(&oldest);
            }
            self.handles.insert(
                path.to_path_buf(),
                Handle {
                    file,
                    len,
                    used: tick,
                },
            );
        }
        Ok(self.handles.get_mut(path).map(|h| {
            h.used = tick;
            (&h.file, h.len)
        }))
    }

    /// A year index: its two slots read, the newest valid kept; `None` when
    /// the series has no index for the year. O(1): 2 reads of 8,812 bytes
    /// (`crates/store/benches/ratio.rs`, C-VD-02).
    ///
    /// # Errors
    ///
    /// [`VerdictError::IndexShort`], [`VerdictError::IndexUnreadable`] and
    /// the host's.
    pub fn year(
        &mut self,
        series: &Series<'_>,
        year: u16,
    ) -> Result<Option<YearIndex>, VerdictError> {
        let mut cost = Cost::default();
        let path = series.year_index(year);
        let Some((file, len)) = self.handle(&path, &mut cost)? else {
            return Ok(None);
        };
        if len < INDEX_LEN_U64 {
            return Err(VerdictError::IndexShort { path, len });
        }
        read_year(file, &path, year, &mut cost).map(Some)
    }

    /// The verdict of one key on one day: at most 5 positional reads and 2
    /// opens (`crates/store/tests/verdict.rs`,
    /// `a_verdict_is_found_by_one_bounded_probe_at_every_day_size`, IA-VD-01;
    /// bench C-VD-01). A key with no record gets its own answer from the day
    /// block's state ([`Found::NoRecord`], IA-VD-16), never the day's worst
    /// code, and costs no more reads.
    ///
    /// # Errors
    ///
    /// [`VerdictError::ImageMissing`] when the index commits an image that
    /// is absent, every header, day-block and record check, and the host's.
    pub fn lookup(
        &mut self,
        series: &Series<'_>,
        day: Day,
        key: &Key,
    ) -> Result<Lookup, VerdictError> {
        let mut cost = Cost::default();
        let Some(entry) = self.day_entry(series, day, &mut cost)? else {
            return Ok(Lookup { cost, answer: None });
        };
        let path = series.image_path(day, entry.generation);
        let (header, block, geo) = self.prefix(series, day, entry.generation, &path, &mut cost)?;
        let Some((file, _)) = self.handle(&path, &mut cost)? else {
            return Err(VerdictError::ImageMissing { path });
        };
        let home = home_of(key.hash, geo.capacity);
        let mut window = [0u8; SLOT_LEN * MAX_PROBE];
        file.read_exact_at(&mut window, off(TABLE_OFFSET + SLOT_LEN * home))
            .map_err(|why| io_error(&path, "read", why))?;
        cost.reads += 1;
        let mut found = Found::NoRecord(NoRecord::of(&block));
        if let Some(ordinal) = first_tagged(&window, key.tag()) {
            if ordinal >= usize_of(u64::from(header.records)) {
                return Err(corrupt(
                    &path,
                    format!("a slot names record {ordinal}, past the records"),
                ));
            }
            let mut raw = [0u8; RECORD_STRIDE];
            file.read_exact_at(&mut raw, off(geo.records_at + RECORD_STRIDE * ordinal))
                .map_err(|why| io_error(&path, "read", why))?;
            cost.reads += 1;
            let ordinal = u32_of(ordinal, &path, "ordinal")?;
            let record = decode_record(&raw, &path, ordinal)?;
            if record.key.hash == key.hash && record.key.text == key.text {
                found = Found::Record { ordinal, record };
            }
        }
        Ok(Lookup {
            cost,
            answer: Some(Answer {
                header,
                day: block,
                found,
            }),
        })
    }

    /// The committed entry of a day, or `None` when it has none.
    fn day_entry(
        &mut self,
        series: &Series<'_>,
        day: Day,
        cost: &mut Cost,
    ) -> Result<Option<DayEntry>, VerdictError> {
        let year = day.year();
        let path = series.year_index(year);
        let Some((file, len)) = self.handle(&path, cost)? else {
            return Ok(None);
        };
        if len < INDEX_LEN_U64 {
            return Err(VerdictError::IndexShort { path, len });
        }
        let index = read_year(file, &path, year, cost)?;
        let entry = index.entry(day);
        Ok((entry.generation > 0).then_some(entry))
    }

    /// The image's header and day block (1 read of 512 bytes), checked
    /// against the series, the day, the generation and the file's length.
    fn prefix(
        &mut self,
        series: &Series<'_>,
        day: Day,
        generation: u32,
        path: &Path,
        cost: &mut Cost,
    ) -> Result<(ImageHeader, DayBlock, Geometry), VerdictError> {
        let Some((file, len)) = self.handle(path, cost)? else {
            return Err(VerdictError::ImageMissing {
                path: path.to_path_buf(),
            });
        };
        let mut bytes = [0u8; TABLE_OFFSET];
        file.read_exact_at(&mut bytes, 0)
            .map_err(|why| io_error(path, "read", why))?;
        cost.reads += 1;
        let header = decode_header(&bytes, path)?;
        let block = decode_day(bytes.get(HEADER_LEN..).unwrap_or_default(), path)?;
        let geo = geometry(&header, path)?;
        let same = header.feed == series.feed
            && header.exchange == series.exchange
            && header.segment == series.segment
            && header.timeframe == series.timeframe
            && header.epoch_day == epoch_day(day)?
            && header.generation == generation;
        if !same {
            return Err(corrupt(
                path,
                "the header names another series, day or generation",
            ));
        }
        if len != off(geo.len) {
            return Err(corrupt(
                path,
                format!("{len} bytes where the header implies {}", geo.len),
            ));
        }
        Ok((header, block, geo))
    }

    /// One page of one code's records of a day's current image: one read of
    /// at most `len` records (O(page)). `None` when the day has no
    /// generation.
    ///
    /// # Errors
    ///
    /// As [`Reader::lookup`].
    pub fn page(
        &mut self,
        series: &Series<'_>,
        day: Day,
        code: Code,
        start: u32,
        len: u32,
    ) -> Result<Option<Vec<Record>>, VerdictError> {
        let mut cost = Cost::default();
        let Some(entry) = self.day_entry(series, day, &mut cost)? else {
            return Ok(None);
        };
        let path = series.image_path(day, entry.generation);
        let (header, _, geo) = self.prefix(series, day, entry.generation, &path, &mut cost)?;
        let (first, count) = header.slice_of(code);
        let from = start.min(count);
        let take = len.min(count - from);
        let mut bytes = vec![0u8; RECORD_STRIDE * usize_of(u64::from(take))];
        let Some((file, _)) = self.handle(&path, &mut cost)? else {
            return Err(VerdictError::ImageMissing { path });
        };
        let ordinal0 = usize_of(u64::from(first + from));
        file.read_exact_at(&mut bytes, off(geo.records_at + RECORD_STRIDE * ordinal0))
            .map_err(|why| io_error(&path, "read", why))?;
        bytes
            .chunks_exact(RECORD_STRIDE)
            .enumerate()
            .map(|(i, raw)| decode_record(raw, &path, u32_of(ordinal0 + i, &path, "ordinal")?))
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    /// The details of a record (or of the day, for `None`) of a day's
    /// generation: two reads per entry it owns (its 8-byte head, then its
    /// payload) and one more head, so the bytes read are its own entries,
    /// bounded by the 4 MiB section; `/verify/day.json` only.
    ///
    /// # Errors
    ///
    /// As [`Reader::lookup`].
    pub fn details(
        &mut self,
        series: &Series<'_>,
        day: Day,
        generation: u32,
        record: Option<(u32, &Record)>,
    ) -> Result<Vec<Detail>, VerdictError> {
        let mut cost = Cost::default();
        let path = series.image_path(day, generation);
        let (header, _, _) = self.prefix(series, day, generation, &path, &mut cost)?;
        let (owner, from) = match record {
            None => (0u32, 0usize),
            Some((_, r)) if r.detail_at == 0 => return Ok(Vec::new()),
            Some((ordinal, r)) => (ordinal + 1, usize_of(u64::from(r.detail_at - 1))),
        };
        if header.detail_bytes == 0 {
            return Ok(Vec::new());
        }
        let section_len = usize_of(u64::from(header.detail_bytes));
        let section_at = u64::from(header.detail_offset);
        let Some((file, _)) = self.handle(&path, &mut cost)? else {
            return Err(VerdictError::ImageMissing { path });
        };
        // Entry by entry: a head, then its payload, while the owner is this
        // one. So the bytes read are this owner's entries and one more head,
        // never the rest of the section (a page of records reads only its
        // own details, C-VD-06).
        let mut out = Vec::new();
        let mut at = from;
        while at + DETAIL_HEAD_LEN <= section_len {
            let mut head = [0u8; DETAIL_HEAD_LEN];
            file.read_exact_at(&mut head, section_at + off(at))
                .map_err(|why| io_error(&path, "read", why))?;
            if u32_at(&head, 0) != owner {
                break;
            }
            let size = usize::from(u16_at(&head, 6));
            if at + DETAIL_HEAD_LEN + size > section_len {
                return Err(corrupt(&path, "a detail entry runs past the section"));
            }
            let mut payload = vec![0u8; size];
            file.read_exact_at(&mut payload, section_at + off(at + DETAIL_HEAD_LEN))
                .map_err(|why| io_error(&path, "read", why))?;
            out.push(Detail {
                kind: u16_at(&head, 4),
                payload,
            });
            at += DETAIL_HEAD_LEN + size;
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;
