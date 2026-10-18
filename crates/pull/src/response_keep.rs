//! Keeping the vendor's answer: what a broker actually sent, beside what the
//! pull landed from it (IA-8; D-4810, D-4859, D-4920; findings F7, DB-G46).
//!
//! # Why this exists
//!
//! Until this module a Zerodha answer was decoded and dropped
//! (`http::HttpSource::window_async`), so a stored minute could never be
//! checked against what the vendor said, and "the vendor sent nothing" could
//! not be told from "the minute was never asked" or "the landing refused the
//! row". The body is now kept, deflated, in one pack per
//! (vendor, instrument[, contract], timeframe, month), with a fixed 128-byte
//! receipt per window, and every row the landing refused is kept with its
//! cause, so a stored hole is explained minute by minute.
//!
//! # The files (`docs/02-store-format.md` §36)
//!
//! ```text
//! <store>/responses/<vendor>/<EXCH>/<SEG>/<SYMBOL>[/<contract>]/<tf>/
//!     <yyyy-mm>.rsb        deflated bodies, appended (segment 0)
//!     <yyyy-mm>.rsr        128-byte receipts, appended
//!     <yyyy-mm>.rsx        24-byte refused-row records, appended (DB-G46)
//!     <yyyy-mm>.<n>.rs?    segment n (1..=7) once segment n-1 is full
//!     keep.lock            the directory's one lock (D-4920, critic IA-8-2)
//! ```
//!
//! The directory mirrors the bar path (`store::path::StorePath::for_key`, the
//! same validation and the same contract segment), so two contracts of one
//! underlying keep two packs. A window's body goes into its FIRST month's pack;
//! every further month it covers gets a receipt only (`kept=3`) that names
//! where the body is. Such a window is all-or-detectable: an `open` record in
//! the directory's [`WINDOWS_NAME`] comes before its first receipt and a
//! `closed` record, its commit, after its last pointer; the next append
//! finishes one an interrupted append left open ([`unfinished`] names it
//! meanwhile; repair round 3).
//!
//! # The rules
//!
//! * **Never a header, never a token.** The caller hands in the body text and
//!   nothing else; [`Answered`] has no field that could hold a header.
//! * **Order: body, refused rows, receipt**, each synced (`F_FULLFSYNC` on
//!   macOS). A reader trusts receipts only: bytes past what the last valid
//!   receipt vouches for, a receipt file whose length is not a multiple of 128,
//!   and a last receipt whose CRC fails are an interrupted append, and the next
//!   append overwrites them (D-4865, the `store/src/provenance.rs` rule). A bad
//!   CRC on an EARLIER receipt is damage, refused by name, never overwritten.
//! * **One writer per pack directory** behind `keep.lock`: one `try_lock`
//!   uncontended; contended, a blocking `flock` on one named waiter thread,
//!   bounded by [`LOCK_WAIT`] and then refused `lock_stalled` (the D-4924
//!   bound; a poll starved, see `lock_within`): two landings of one month
//!   never interleave a pack.
//! * **Caps, named.** A segment holds at most [`MAX_PACK_BYTES`] of bodies,
//!   [`MAX_RECEIPTS`] receipts and [`REFUSAL_ROOM`] refused-row records. A
//!   segment that cannot take the next window rolls to the next one, with an
//!   audited error event naming it (the rules audit's RA-075: keep on rolling,
//!   alert on the first full pack). In the last of [`MAX_SEGMENTS`] segments a
//!   body that does not fit writes a receipt `kept=0 reason=pack_full` (its
//!   refused rows are still kept); with [`MAX_RECEIPTS`] receipts there, or no
//!   room for the window's refused rows, nothing is written and the refusal
//!   `receipts_full` or `refusals_full` names the pack, its audited error
//!   event counting the rows left unkept. Nothing is dropped silently.
//! * **A repeated body is kept once** (IA-RK-06): a body whose BLAKE3 a
//!   receipt of the newest segment already holds writes a receipt only,
//!   `kept=2 reason=duplicate_body`, naming that receipt.
//! * **A refused answer is kept too** (RA-299): a 2xx answer that was a
//!   refusal or did not decode (a login wall, a maintenance page) keeps its
//!   first [`REFUSED_HEAD_BYTES`] bytes with its status, `kept=4`.
//!
//! # Cost (`docs/06-limits.md` L-IA-6, L-IA-48 to L-IA-52)
//!
//! An append is O(body) to deflate, one read of the newest segment's receipts
//! (at most [`MAX_RECEIPTS`] × 128 B = 128 KiB, the bound the duplicate check
//! also uses), at most [`MAX_SEGMENTS`] `stat`s to find that segment, two or
//! three positional writes and as many syncs (the refused rows only when the
//! landing refused any; a directory sync when a file takes its first bytes),
//! plus one receipt per further month the window covers and, for a window
//! over months, two 64-byte window-journal records. Every append also reads
//! the journal's last one or two records (one `stat`, at most 128 bytes) and
//! finishes a window an interrupted append left open (L-IA-52). Nothing depends on how many
//! packs the store holds: C-RK-01
//! (`pull::bench::keep_lookup_is_flat`) and C-RK-02
//! (`pull::bench::keep_append_is_flat`) measure both at 1x, 10x and 100x
//! packs in the store.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{FileExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use brutex_core::instrument::{Contract, InstrumentKey};
use brutex_core::vendor::Vendor;
use store::path::{FileKind, PathError, StorePath, Timeframe, YearMonth};

use crate::fetch::{FetchError, RawWindow, RowCause};
use crate::session::{IST_OFFSET_SECS, Window};

/// One receipt's length on disk.
pub const RECEIPT_LEN: usize = 128;
/// One refused-row record's length on disk.
pub const REFUSAL_LEN: usize = 24;
/// The most body bytes (deflated) one pack segment holds: 64 MiB.
pub const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;
/// The most refused-row records one segment's `.rsx` holds: 64 MiB of
/// [`REFUSAL_LEN`]-byte records, 2,796,202 (DB-G46). A window whose rows do
/// not fit rolls the month to the next segment like a full body; with no
/// segment able to take them, nothing is written and the append is refused
/// `refusals_full`. A refused row is never dropped silently.
pub const REFUSAL_ROOM: u64 = MAX_PACK_BYTES / REFUSAL_LEN as u64;
/// The most receipts one pack segment holds.
pub const MAX_RECEIPTS: u32 = 1_024;
/// The most ABANDONED `keep-lock` waiter threads one pack lock may have
/// parked before a new contended append is refused at once. A waiter whose
/// caller's wait ran out (refused `lock_stalled`) stays parked in `flock`
/// until the holder lets go; against a holder that never does, this bounds
/// the threads and descriptors left behind, and the next contended append is
/// refused `lock_stalled` at once, naming how many are parked. A waiter whose
/// caller is still inside its wait is never counted, so healthy contention
/// waits up to [`LOCK_WAIT`] however many appends queue (repair round 2). The
/// waits already in flight when the fourth is abandoned can each still run
/// out, so the threads left behind are at most this plus the appends that
/// were waiting at that moment.
pub const MAX_PARKED_WAITERS: usize = 4;
/// The most segments one month's pack rolls through.
pub const MAX_SEGMENTS: u8 = 8;
/// The most bytes of a refused answer that are kept.
pub const REFUSED_HEAD_BYTES: usize = 4_096;
/// The longest an append waits for the pack lock before it is refused.
pub const LOCK_WAIT: Duration = Duration::from_secs(30);
/// The directory under the store root that holds every kept answer.
pub const KEEP_ROOT: &str = "responses";

/// What a pull's request answered: the decoded window and the body it was
/// decoded from (D-4920: returned by `HttpSource::window_answered_async`).
///
/// **No header field exists**, so no header or token can reach a pack.
#[derive(Debug, Clone)]
pub struct Answered {
    /// The rows, exactly as `window_async` returns them.
    pub window: RawWindow,
    /// The body text, byte for byte as the vendor sent it.
    pub body: Arc<str>,
    /// BLAKE3 of [`Self::body`].
    pub digest: [u8; 32],
    /// The HTTP status the answer carried (a success).
    pub status: u16,
}

impl Answered {
    /// Wraps a decoded window with the body it came from.
    #[must_use]
    pub fn of(window: RawWindow, body: String, status: u16) -> Self {
        let digest = brutex_core::blake3::hash(body.as_bytes());
        Self {
            window,
            body: Arc::from(body),
            digest,
            status,
        }
    }
}

/// The head of a 2xx answer that was a refusal or did not decode (RA-299).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedAnswer {
    /// The HTTP status it carried.
    pub status: u16,
    /// Its first [`REFUSED_HEAD_BYTES`] bytes, cut at a character boundary.
    pub head: Arc<str>,
    /// Its whole length in bytes.
    pub len: u64,
}

impl RefusedAnswer {
    /// Keeps the head of `text`.
    #[must_use]
    pub fn of(status: u16, text: &str) -> Self {
        let mut cut = text.len().min(REFUSED_HEAD_BYTES);
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        Self {
            status,
            head: Arc::from(text.get(..cut).unwrap_or_default()),
            len: u64::try_from(text.len()).unwrap_or(u64::MAX),
        }
    }
}

/// A request that did not produce an [`Answered`]: the error `window_async`
/// returns, and the head of the body when one arrived under a success status.
#[derive(Debug)]
pub struct Unanswered {
    /// The refusal, exactly as `window_async` names it.
    pub error: FetchError,
    /// The body's head, when a 2xx body arrived and was refused.
    pub refused: Option<RefusedAnswer>,
}

impl From<FetchError> for Unanswered {
    fn from(error: FetchError) -> Self {
        Self {
            error,
            refused: None,
        }
    }
}

impl From<Unanswered> for FetchError {
    fn from(unanswered: Unanswered) -> Self {
        unanswered.error
    }
}

/// Whether a receipt's body is in its pack, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// No body was kept; [`Reason`] says why.
    NotKept,
    /// The body is in this pack.
    Kept,
    /// The same body as an earlier receipt of this segment (`body_of`).
    Duplicate,
    /// The window's body is in its first month's pack (`body_of`, `segment`).
    InFirstMonth,
    /// The head of a refused answer is in this pack.
    RefusedHead,
}

impl Kept {
    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::NotKept => 0,
            Self::Kept => 1,
            Self::Duplicate => 2,
            Self::InFirstMonth => 3,
            Self::RefusedHead => 4,
        }
    }

    /// The value of a byte on disk, or `None` for one no writer writes.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::NotKept),
            1 => Some(Self::Kept),
            2 => Some(Self::Duplicate),
            3 => Some(Self::InFirstMonth),
            4 => Some(Self::RefusedHead),
            _ => None,
        }
    }
}

/// Why a receipt says what it says about its body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Kept as offered.
    Kept,
    /// The last segment had no room for the body.
    PackFull,
    /// The disk refused the body.
    StorageFull,
    /// The body is already in the segment.
    DuplicateBody,
    /// A further month of a window whose body is in its first month.
    LaterMonth,
    /// The answer was refused; its head is kept.
    RefusedAnswer,
}

impl Reason {
    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Kept => 0,
            Self::PackFull => 1,
            Self::StorageFull => 2,
            Self::DuplicateBody => 3,
            Self::LaterMonth => 4,
            Self::RefusedAnswer => 5,
        }
    }

    /// The value of a byte on disk, or `None` for one no writer writes.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Kept),
            1 => Some(Self::PackFull),
            2 => Some(Self::StorageFull),
            3 => Some(Self::DuplicateBody),
            4 => Some(Self::LaterMonth),
            5 => Some(Self::RefusedAnswer),
            _ => None,
        }
    }

    /// The word pages and events use.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::PackFull => "pack_full",
            Self::StorageFull => "storage_full",
            Self::DuplicateBody => "duplicate_body",
            Self::LaterMonth => "later_month",
            Self::RefusedAnswer => "refused_answer",
        }
    }
}

/// Which landing door the window went through (round 3, SF-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    /// `ingest::from_window`.
    FromWindow,
    /// `ingest::repair_window`.
    RepairWindow,
}

impl Door {
    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::FromWindow => 1,
            Self::RepairWindow => 2,
        }
    }

    /// The value of a byte on disk.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::FromWindow),
            2 => Some(Self::RepairWindow),
            _ => None,
        }
    }
}

/// One 128-byte receipt (`docs/02-store-format.md` §36), little-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    /// Offset 0: the asked window's first second (unix s, IST midnight).
    pub asked_from: i64,
    /// Offset 8: the asked window's last second (unix s).
    pub asked_to: i64,
    /// Offset 16: the first decoded row's stamp (unix s; 0 = none). For
    /// `kept=4`, the refused answer's whole length in bytes.
    pub first_stamp: i64,
    /// Offset 24: the last decoded row's stamp (unix s; 0 = none).
    pub last_stamp: i64,
    /// Offset 32: where the body starts in its `.rsb`.
    pub body_offset: u64,
    /// Offset 40: the body's deflated length.
    pub deflated_len: u32,
    /// Offset 44: the body's raw length.
    pub raw_len: u32,
    /// Offset 48: rows decoded from the answer.
    pub rows_decoded: u32,
    /// Offset 52: rows the landing appended.
    pub rows_landed: u32,
    /// Offset 56: rows the landing refused by name (D-3351).
    pub rows_refused: u32,
    /// Offset 60: the run id from the runs ledger (D-4880).
    pub run: u32,
    /// Offset 64: the HTTP status.
    pub status: u16,
    /// Offset 66: the rung asked (`vendor::Granularity` as a byte).
    pub rung: u8,
    /// Offset 67: whether and where the body is kept.
    pub kept: Kept,
    /// Offset 68: why.
    pub reason: Reason,
    /// Offset 69: the landing rules version (D-4843).
    pub rules_version: u8,
    /// Offset 70: the landing door.
    pub door: Door,
    /// Offset 71: the segment of the pack that holds the body and the refused
    /// rows (this receipt's own, except for `kept=3`).
    pub segment: u8,
    /// Offset 72: BLAKE3 of the body kept (the head, for `kept=4`).
    pub digest: [u8; 32],
    /// Offset 104: `store::verdict::key_hash` of the instrument key.
    pub key_hash: u64,
    /// Offset 112: the ordinal of the receipt that holds the body.
    pub body_of: u32,
    /// Offset 116: the first refused-row record's index in the `.rsx`.
    pub refusals_from: u32,
    /// Offset 120: how many refused-row records this receipt owns.
    pub refusals: u32,
}

impl Receipt {
    /// The 128 bytes, CRC-32C over bytes 0..124 at offset 124.
    #[must_use]
    pub fn encode(&self) -> [u8; RECEIPT_LEN] {
        let mut out = [0_u8; RECEIPT_LEN];
        let mut at = 0;
        let mut put = |bytes: &[u8]| {
            if let Some(slot) = out.get_mut(at..at + bytes.len()) {
                slot.copy_from_slice(bytes);
            }
            at += bytes.len();
        };
        put(&self.asked_from.to_le_bytes());
        put(&self.asked_to.to_le_bytes());
        put(&self.first_stamp.to_le_bytes());
        put(&self.last_stamp.to_le_bytes());
        put(&self.body_offset.to_le_bytes());
        put(&self.deflated_len.to_le_bytes());
        put(&self.raw_len.to_le_bytes());
        put(&self.rows_decoded.to_le_bytes());
        put(&self.rows_landed.to_le_bytes());
        put(&self.rows_refused.to_le_bytes());
        put(&self.run.to_le_bytes());
        put(&self.status.to_le_bytes());
        put(&[
            self.rung,
            self.kept.code(),
            self.reason.code(),
            self.rules_version,
            self.door.code(),
            self.segment,
        ]);
        put(&self.digest);
        put(&self.key_hash.to_le_bytes());
        put(&self.body_of.to_le_bytes());
        put(&self.refusals_from.to_le_bytes());
        put(&self.refusals.to_le_bytes());
        let crc = store::crc::crc32c(out.get(..124).unwrap_or_default());
        if let Some(slot) = out.get_mut(124..) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        out
    }

    /// Reads 128 bytes back, or `None` when the CRC or a coded byte is wrong.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; RECEIPT_LEN] = bytes.try_into().ok()?;
        let crc = u32::from_le_bytes(take(bytes, 124)?);
        if store::crc::crc32c(bytes.get(..124)?) != crc {
            return None;
        }
        Some(Self {
            asked_from: i64::from_le_bytes(take(bytes, 0)?),
            asked_to: i64::from_le_bytes(take(bytes, 8)?),
            first_stamp: i64::from_le_bytes(take(bytes, 16)?),
            last_stamp: i64::from_le_bytes(take(bytes, 24)?),
            body_offset: u64::from_le_bytes(take(bytes, 32)?),
            deflated_len: u32::from_le_bytes(take(bytes, 40)?),
            raw_len: u32::from_le_bytes(take(bytes, 44)?),
            rows_decoded: u32::from_le_bytes(take(bytes, 48)?),
            rows_landed: u32::from_le_bytes(take(bytes, 52)?),
            rows_refused: u32::from_le_bytes(take(bytes, 56)?),
            run: u32::from_le_bytes(take(bytes, 60)?),
            status: u16::from_le_bytes(take(bytes, 64)?),
            rung: *bytes.get(66)?,
            kept: Kept::from_code(*bytes.get(67)?)?,
            reason: Reason::from_code(*bytes.get(68)?)?,
            rules_version: *bytes.get(69)?,
            door: Door::from_code(*bytes.get(70)?)?,
            segment: *bytes.get(71)?,
            digest: take(bytes, 72)?,
            key_hash: u64::from_le_bytes(take(bytes, 104)?),
            body_of: u32::from_le_bytes(take(bytes, 112)?),
            refusals_from: u32::from_le_bytes(take(bytes, 116)?),
            refusals: u32::from_le_bytes(take(bytes, 120)?),
        })
    }
}

fn take<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    bytes.get(at..at + N)?.try_into().ok()
}

/// One row the landing refused, with its cause (DB-G46): the record that
/// lets a stored hole be explained minute by minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefusedRow {
    /// The row's stamp as the landing judged it: IST local seconds since
    /// 1970 (`fetch::Refused::local_seconds`).
    pub local_second: i64,
    /// Why it was refused.
    pub cause: RowCause,
}

/// The byte a cause is written as. Append-only: a new cause takes a new byte.
#[must_use]
pub const fn cause_code(cause: RowCause) -> u8 {
    match cause {
        RowCause::Timestamp => 1,
        RowCause::CashEligibility => 2,
        RowCause::Price => 3,
        RowCause::OffGrid => 4,
        RowCause::Conflicting => 5,
        RowCause::BeforeFirstDayBar => 6,
        RowCause::ForeignLevel => 7,
    }
}

/// The cause a byte names, or `None` for one no writer writes.
#[must_use]
pub const fn cause_of(code: u8) -> Option<RowCause> {
    match code {
        1 => Some(RowCause::Timestamp),
        2 => Some(RowCause::CashEligibility),
        3 => Some(RowCause::Price),
        4 => Some(RowCause::OffGrid),
        5 => Some(RowCause::Conflicting),
        6 => Some(RowCause::BeforeFirstDayBar),
        7 => Some(RowCause::ForeignLevel),
        _ => None,
    }
}

/// A landing's refusal record whose two per-row lists are not one entry per
/// row each: a caller took the seconds (the gap ledger's) and not the causes,
/// or the other way round. Refused by name, never zipped short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListsDiffer {
    /// Refused rows' seconds the record holds.
    pub seconds: usize,
    /// Refused rows' causes the record holds.
    pub causes: usize,
}

impl std::fmt::Display for ListsDiffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the landing's refusal record holds {} refused seconds and {} causes; \
             a row's cause cannot be named, so no row is recorded",
            self.seconds, self.causes
        )
    }
}

impl std::error::Error for ListsDiffer {}

impl RefusedRow {
    /// Every row one landing refused, each with its OWN cause, in answer
    /// order, read from the landing's own record (DB-G46). Every landing
    /// refuses through `fetch::Refused::note`, so this is the one path for
    /// every vendor, door and instrument. O(rows refused).
    ///
    /// # Errors
    ///
    /// [`ListsDiffer`] when the record's seconds and causes are not one per
    /// row each.
    pub fn all_of(refused: &crate::fetch::Refused) -> Result<Vec<Self>, ListsDiffer> {
        if refused.local_seconds.len() != refused.causes.len() {
            return Err(ListsDiffer {
                seconds: refused.local_seconds.len(),
                causes: refused.causes.len(),
            });
        }
        Ok(refused
            .local_seconds
            .iter()
            .zip(&refused.causes)
            .map(|(&local_second, &cause)| Self {
                local_second,
                cause,
            })
            .collect())
    }

    /// 24 bytes: stamp, owning receipt's ordinal, cause, zeros, CRC-32C over
    /// bytes 0..20.
    #[must_use]
    pub fn encode(&self, ordinal: u32) -> [u8; REFUSAL_LEN] {
        let mut out = [0_u8; REFUSAL_LEN];
        if let Some(slot) = out.get_mut(0..8) {
            slot.copy_from_slice(&self.local_second.to_le_bytes());
        }
        if let Some(slot) = out.get_mut(8..12) {
            slot.copy_from_slice(&ordinal.to_le_bytes());
        }
        if let Some(slot) = out.get_mut(12) {
            *slot = cause_code(self.cause);
        }
        let crc = store::crc::crc32c(out.get(..20).unwrap_or_default());
        if let Some(slot) = out.get_mut(20..24) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        out
    }

    /// Reads 24 bytes back with the owning receipt's ordinal, or `None`.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<(Self, u32)> {
        let bytes: &[u8; REFUSAL_LEN] = bytes.try_into().ok()?;
        if store::crc::crc32c(bytes.get(..20)?) != u32::from_le_bytes(take(bytes, 20)?) {
            return None;
        }
        let row = Self {
            local_second: i64::from_le_bytes(take(bytes, 0)?),
            cause: cause_of(*bytes.get(12)?)?,
        };
        Some((row, u32::from_le_bytes(take(bytes, 8)?)))
    }
}

/// Which pack: the bar path's own key, so every vendor, instrument and
/// contract takes one path (no special case).
#[derive(Debug, Clone, Copy)]
pub struct Pack<'a> {
    /// Whose answer.
    pub vendor: Vendor,
    /// Which instrument (and contract).
    pub key: &'a InstrumentKey,
    /// Which rung.
    pub timeframe: Timeframe,
}

/// What the landing did with the answer, written into the receipt.
#[derive(Debug, Clone, Copy)]
pub struct Landing<'a> {
    /// The window asked for.
    pub window: Window,
    /// The first decoded row's stamp (unix s; 0 = none).
    pub first_stamp: i64,
    /// The last decoded row's stamp (unix s; 0 = none).
    pub last_stamp: i64,
    /// Rows decoded.
    pub rows_decoded: u32,
    /// Rows the landing appended.
    pub rows_landed: u32,
    /// Rows the landing refused by name.
    pub rows_refused: u32,
    /// Each refused row with its cause, when the caller has them.
    pub refused: &'a [RefusedRow],
    /// The runs-ledger id.
    pub run: u32,
    /// The rung asked (`vendor::Granularity` as a byte).
    pub rung: u8,
    /// The landing rules version.
    pub rules_version: u8,
    /// The door.
    pub door: Door,
}

/// What is offered for keeping.
#[derive(Debug, Clone, Copy)]
pub enum Offered<'a> {
    /// A decoded answer.
    Answer(&'a Answered),
    /// The head of a refused answer.
    Refused(&'a RefusedAnswer),
}

/// Where one receipt went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// The month's pack.
    pub month: YearMonth,
    /// The segment.
    pub segment: u8,
    /// The receipt's ordinal in that segment.
    pub ordinal: u32,
    /// The receipt as written.
    pub receipt: Receipt,
    /// Whether this append rolled the month to a new segment.
    pub rolled: bool,
}

/// What an append wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The first month's receipt (the body's).
    pub first: Placed,
    /// One receipt per further month the window covers.
    pub further: Vec<Placed>,
}

/// Why nothing (more) was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepError {
    /// The key cannot be a store path.
    Path(PathError),
    /// The last segment already holds [`MAX_RECEIPTS`] receipts.
    ReceiptsFull {
        /// The receipts file.
        pack: PathBuf,
    },
    /// No segment of the month can take the window's refused-row records:
    /// the last segment's `.rsx` holds too many already, or the window
    /// refused more rows than any segment holds ([`REFUSAL_ROOM`]). Nothing
    /// is written: the rows are refused by name, never dropped.
    RefusalsFull {
        /// The refused-rows file that cannot take them.
        pack: PathBuf,
        /// Records it already holds.
        held: u32,
        /// Records this window brought.
        rows: u64,
    },
    /// Another writer held the month's lock past [`LOCK_WAIT`].
    LockStalled {
        /// The lock file.
        lock: PathBuf,
        /// Milliseconds waited before the refusal.
        waited_ms: u64,
    },
    /// At least [`MAX_PARKED_WAITERS`] earlier waits on this lock ran out and their
    /// waiters are still parked on a holder that has not let go since; this
    /// append waited for nothing.
    LockWaitersParked {
        /// The lock file.
        lock: PathBuf,
        /// Abandoned waiter threads parked on it.
        parked: usize,
    },
    /// A receipt before the last one fails its CRC: damage, not a torn tail.
    Damaged {
        /// The file.
        path: PathBuf,
        /// The first bad receipt or record.
        ordinal: u64,
    },
    /// Something other than a regular file sits at a pack's name.
    NotRegular {
        /// The name.
        path: PathBuf,
    },
    /// The pack's window journal already holds [`MAX_WINDOW_RECORDS`]
    /// records, so a window over months cannot be made all-or-detectable;
    /// nothing is written (repair round 3).
    WindowsFull {
        /// The journal.
        journal: PathBuf,
    },
    /// The body is larger than `http::MAX_RESPONSE_BYTES` (§8 Z7).
    TooLarge {
        /// Its length.
        len: u64,
    },
    /// A kept body does not inflate to what its receipt says.
    BodyMismatch {
        /// The file.
        path: PathBuf,
        /// What differs.
        what: &'static str,
    },
    /// A receipt says no body was kept.
    NotKept {
        /// Why.
        reason: Reason,
    },
    /// The host refused an operation.
    Io {
        /// What was being done.
        op: &'static str,
        /// On which file.
        path: PathBuf,
        /// The host's kind.
        kind: io::ErrorKind,
    },
}

impl KeepError {
    /// The word pages, events and the verdict's detail use.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Path(_) => "path_refused",
            Self::ReceiptsFull { .. } => "receipts_full",
            Self::RefusalsFull { .. } => "refusals_full",
            Self::LockStalled { .. } | Self::LockWaitersParked { .. } => "lock_stalled",
            Self::Damaged { .. } => "pack_damaged",
            Self::NotRegular { .. } => "not_a_regular_file",
            Self::WindowsFull { .. } => "windows_full",
            Self::TooLarge { .. } => "source_refused",
            Self::BodyMismatch { .. } => "body_mismatch",
            Self::NotKept { .. } => "body_not_kept",
            Self::Io { .. } => "io",
        }
    }

    fn io(op: &'static str, path: &Path, why: &io::Error) -> Self {
        Self::Io {
            op,
            path: path.to_path_buf(),
            kind: why.kind(),
        }
    }
}

impl std::fmt::Display for KeepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(why) => write!(f, "path_refused: {why}"),
            Self::ReceiptsFull { pack } => write!(
                f,
                "receipts_full: {} holds {MAX_RECEIPTS} receipts in its last segment; nothing written",
                pack.display()
            ),
            Self::RefusalsFull { pack, held, rows } => write!(
                f,
                "refusals_full: {} holds {held} refused-row records of at most {REFUSAL_ROOM} and no segment can take {rows} more; nothing written",
                pack.display()
            ),
            Self::LockWaitersParked { lock, parked } => write!(
                f,
                "lock_stalled: {} has {parked} abandoned waits still parked on a holder that has not let go (new waits are refused once {MAX_PARKED_WAITERS} are parked; waits already in flight then can add more); nothing written",
                lock.display()
            ),
            Self::LockStalled { lock, waited_ms } => write!(
                f,
                "lock_stalled: {} was held past this append's wait (waited {waited_ms} ms of at most {} s); nothing written",
                lock.display(),
                LOCK_WAIT.as_secs()
            ),
            Self::Damaged { path, ordinal } => write!(
                f,
                "pack_damaged: {} record {ordinal} fails its CRC and is not the last",
                path.display()
            ),
            Self::NotRegular { path } => {
                write!(f, "not_a_regular_file: {}", path.display())
            }
            Self::WindowsFull { journal } => write!(
                f,
                "windows_full: {} holds {MAX_WINDOW_RECORDS} records; a window over months is not kept; nothing written",
                journal.display()
            ),
            Self::TooLarge { len } => write!(
                f,
                "source_refused: a body of {len} bytes is past {} bytes; nothing kept",
                crate::http::MAX_RESPONSE_BYTES
            ),
            Self::BodyMismatch { path, what } => {
                write!(f, "body_mismatch: {} {what}", path.display())
            }
            Self::NotKept { reason } => write!(f, "body_not_kept: {}", reason.word()),
            Self::Io { op, path, kind } => {
                write!(f, "io: {op} {} refused ({kind:?})", path.display())
            }
        }
    }
}

impl std::error::Error for KeepError {}

impl From<PathError> for KeepError {
    fn from(why: PathError) -> Self {
        Self::Path(why)
    }
}

/// The directory a pack lives in, validated by the bar path's own rules.
///
/// # Errors
///
/// [`KeepError::Path`] for a key the store cannot address.
pub fn pack_dir(root: &Path, pack: Pack<'_>) -> Result<PathBuf, KeepError> {
    // THE BAR PATH'S VALIDATION, NOT A SECOND ONE: every segment is checked by
    // `store::path::check_segment` and the contract rendered by `Contract::of`.
    let month = YearMonth::new(2000, 1)?;
    StorePath::for_key(pack.vendor, pack.key, pack.timeframe, month, FileKind::Bars)?;
    let mut dir = root.join(KEEP_ROOT);
    dir.push(pack.vendor.as_str());
    dir.push(pack.key.exchange.as_str());
    dir.push(pack.key.segment.as_str());
    dir.push(pack.key.underlying.as_str());
    if let Some(contract) = Contract::of(pack.key.kind) {
        dir.push(contract.as_str());
    }
    dir.push(pack.timeframe.as_str());
    Ok(dir)
}

/// The three files of one month's segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    /// Bodies.
    pub bodies: PathBuf,
    /// Receipts.
    pub receipts: PathBuf,
    /// Refused-row records.
    pub refusals: PathBuf,
}

/// The files of `segment` of `month` under `dir`.
#[must_use]
pub fn names(dir: &Path, month: YearMonth, segment: u8) -> Names {
    let stem = if segment == 0 {
        month.to_string()
    } else {
        format!("{month}.{segment}")
    };
    Names {
        bodies: dir.join(format!("{stem}.rsb")),
        receipts: dir.join(format!("{stem}.rsr")),
        refusals: dir.join(format!("{stem}.rsx")),
    }
}

/// The pack directory's lock: ONE per (vendor, instrument, timeframe), not
/// one per month, so a store of 26,617 instrument-months does not carry
/// 26,617 lock files beside its packs (the disk rule), and a window over
/// several months is kept under one lock from its first receipt to its last.
/// Landings of one instrument are serial anyway, so nothing waits that would
/// not have waited.
fn lock_path(dir: &Path) -> PathBuf {
    dir.join(LOCK_NAME)
}

/// The lock file's name in every pack directory.
pub const LOCK_NAME: &str = "keep.lock";

/// The window journal's name in every pack directory (§36, repair round 3):
/// one `open` record before a window over months writes its first month's
/// receipt, one closing record after its last further month's pointer.
pub const WINDOWS_NAME: &str = "windows.rsw";
/// One window-journal record's length on disk.
pub const WINDOW_RECORD_LEN: usize = 64;
/// The most records one pack's window journal holds (64 MiB of them). A
/// window over months that would need more is refused `windows_full` before
/// anything is written; a window inside one month needs none.
pub const MAX_WINDOW_RECORDS: u64 = MAX_PACK_BYTES / WINDOW_RECORD_LEN as u64;

/// What a window-journal record says about its window over months.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    /// Written before the first month's receipt. While no writer holds the
    /// pack lock, an `Open` last record is an interrupted window: its first
    /// month's receipt may stand with no pointer in a later month.
    Open,
    /// Written after the last further month's pointer: the window is whole.
    Closed,
    /// Written by the next append, which found the window open with its
    /// first month's receipt committed and wrote every missing pointer.
    Completed,
    /// Written by the next append, which found the window open with no first
    /// month's receipt: nothing was kept for it and nothing points at it.
    Abandoned,
}

impl WindowState {
    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Open => 1,
            Self::Closed => 2,
            Self::Completed => 3,
            Self::Abandoned => 4,
        }
    }

    /// The value of a byte on disk, or `None` for one no writer writes.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Open),
            2 => Some(Self::Closed),
            3 => Some(Self::Completed),
            4 => Some(Self::Abandoned),
            _ => None,
        }
    }

    /// The word events and pages use.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
}

/// One 64-byte window-journal record (`docs/02-store-format.md` §36),
/// little-endian: which window over months, where its first month's receipt
/// goes, and how far it got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRecord {
    /// Offset 0: the receipt's `asked_from`.
    pub asked_from: i64,
    /// Offset 8: the receipt's `asked_to`; its IST month is the window's last.
    pub asked_to: i64,
    /// Offset 16: the receipt's `digest`.
    pub digest: [u8; 32],
    /// Offset 48: the receipt's `run`.
    pub run: u32,
    /// Offsets 52 (year, `u16`) and 54 (month): the window's first month.
    pub month: YearMonth,
    /// Offset 55: the segment of the first month the receipt goes to.
    pub segment: u8,
    /// Offset 56 (`u16`): the receipt's ordinal there (below
    /// [`MAX_RECEIPTS`]).
    pub ordinal: u16,
    /// Offset 58: the state.
    pub state: WindowState,
    /// Offset 59: for [`WindowState::Completed`], the pointers the next
    /// append wrote (saturating at 255); 0 otherwise.
    pub written: u8,
}

impl WindowRecord {
    /// The 64 bytes, CRC-32C over bytes 0..60 at offset 60.
    #[must_use]
    pub fn encode(&self) -> [u8; WINDOW_RECORD_LEN] {
        let mut out = [0_u8; WINDOW_RECORD_LEN];
        let mut at = 0;
        let mut put = |bytes: &[u8]| {
            if let Some(slot) = out.get_mut(at..at + bytes.len()) {
                slot.copy_from_slice(bytes);
            }
            at += bytes.len();
        };
        put(&self.asked_from.to_le_bytes());
        put(&self.asked_to.to_le_bytes());
        put(&self.digest);
        put(&self.run.to_le_bytes());
        put(&self.month.year().to_le_bytes());
        put(&[self.month.month(), self.segment]);
        put(&self.ordinal.to_le_bytes());
        put(&[self.state.code(), self.written]);
        let crc = store::crc::crc32c(out.get(..60).unwrap_or_default());
        if let Some(slot) = out.get_mut(60..) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        out
    }

    /// Reads 64 bytes back, or `None` when the CRC or a coded byte is wrong.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; WINDOW_RECORD_LEN] = bytes.try_into().ok()?;
        let crc = u32::from_le_bytes(take(bytes, 60)?);
        if store::crc::crc32c(bytes.get(..60)?) != crc {
            return None;
        }
        Some(Self {
            asked_from: i64::from_le_bytes(take(bytes, 0)?),
            asked_to: i64::from_le_bytes(take(bytes, 8)?),
            digest: take(bytes, 16)?,
            run: u32::from_le_bytes(take(bytes, 48)?),
            month: YearMonth::new(u16::from_le_bytes(take(bytes, 52)?), *bytes.get(54)?).ok()?,
            segment: *bytes.get(55)?,
            ordinal: u16::from_le_bytes(take(bytes, 56)?),
            state: WindowState::from_code(*bytes.get(58)?)?,
            written: *bytes.get(59)?,
        })
    }

    /// Whether `receipt` is the first month's receipt this record names.
    fn names(&self, receipt: &Receipt) -> bool {
        receipt.asked_from == self.asked_from
            && receipt.asked_to == self.asked_to
            && receipt.digest == self.digest
            && receipt.run == self.run
    }
}

/// The months a window covers, first to last.
fn months_of(window: Window) -> Result<Vec<YearMonth>, KeepError> {
    months_from(window.from().year_month()?, window.to().year_month()?)
}

/// The months from `first` to `last`, both kept: the one walk the append and
/// the finishing of an interrupted window both take.
fn months_from(first: YearMonth, last: YearMonth) -> Result<Vec<YearMonth>, KeepError> {
    let mut out = vec![first];
    let mut at = first;
    while at < last {
        at = if at.month() == 12 {
            YearMonth::new(at.year().saturating_add(1), 1)?
        } else {
            YearMonth::new(at.year(), at.month() + 1)?
        };
        out.push(at);
    }
    Ok(out)
}

/// IST midnight of the window's first day and the last second of its last
/// day, as unix seconds.
fn asked_span(window: Window) -> (i64, i64) {
    let from = i64::from(window.from().days_from_epoch()) * 86_400 - IST_OFFSET_SECS;
    let to = (i64::from(window.to().days_from_epoch()) + 1) * 86_400 - IST_OFFSET_SECS - 1;
    (from, to)
}

/// The month an asked-from second falls in, in IST.
fn month_of_unix(second: i64) -> Result<YearMonth, KeepError> {
    let days = (second + IST_OFFSET_SECS).div_euclid(86_400);
    // Howard Hinnant's civil_from_days, over integers only.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let refused = || KeepError::Path(PathError::EmptySegment { field: "month" });
    let year = u16::try_from(year).map_err(|_| refused())?;
    let month = u8::try_from(month).map_err(|_| refused())?;
    Ok(YearMonth::new(year, month)?)
}

/// Opens a pack file through the store's one reading door
/// (`store::safe_open`, R3-16, D-4951): never through a final link, never
/// waiting on a pipe, never anything but a regular file (RA-317). A writer
/// opens an existing file through the same door, and creates a missing one
/// with `O_EXCL` and the same flags, so nothing already at the name is ever
/// opened by the create.
fn open_regular(path: &Path, write: bool) -> Result<Option<File>, KeepError> {
    let refused = |why: store::safe_open::ReadRefusal| {
        if why.found().is_some() {
            return KeepError::NotRegular {
                path: path.to_path_buf(),
            };
        }
        let kind = match &why {
            store::safe_open::ReadRefusal::Io { source, .. } => source.kind(),
            _ => io::ErrorKind::Other,
        };
        KeepError::Io {
            op: "open",
            path: path.to_path_buf(),
            kind,
        }
    };
    let existing = if write {
        store::safe_open::open_read_write(path).map(|(file, _len)| file)
    } else {
        store::safe_open::open_read(path)
    };
    match existing {
        Ok(file) => return Ok(Some(file)),
        Err(why) if why.is_missing() && !write => return Ok(None),
        Err(why) if why.is_missing() => {}
        Err(why) => return Err(refused(why)),
    }
    // MISSING, AND THIS IS A WRITER UNDER THE PACK LOCK (or creating the lock
    // itself): born `O_EXCL`, so a name that appeared meanwhile is opened by
    // the door above on the second try, never by this create.
    let created = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .custom_flags(store::open_flags::O_NOFOLLOW | store::open_flags::O_NONBLOCK)
        .open(path);
    match created {
        Ok(file) => Ok(Some(file)),
        Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
            store::safe_open::open_read_write(path)
                .map(|(file, _len)| Some(file))
                .map_err(refused)
        }
        Err(why) => Err(KeepError::io("create", path, &why)),
    }
}

/// Reads a whole small file (at most `cap` bytes; more is damage).
fn read_bounded(file: &File, path: &Path, cap: u64) -> Result<Vec<u8>, KeepError> {
    let len = file
        .metadata()
        .map_err(|why| KeepError::io("stat", path, &why))?
        .len();
    if len > cap {
        return Err(KeepError::Damaged {
            path: path.to_path_buf(),
            ordinal: cap / RECEIPT_LEN as u64,
        });
    }
    let mut out = vec![0_u8; usize::try_from(len).unwrap_or(0)];
    file.read_exact_at(&mut out, 0)
        .map_err(|why| KeepError::io("read", path, &why))?;
    Ok(out)
}

/// The trusted receipts of one segment, and where the next append writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Segment {
    /// Every receipt a reader may trust, in ordinal order.
    pub receipts: Vec<Receipt>,
    /// Bytes past the last trusted receipt (a torn tail), if any.
    pub torn_receipt_bytes: u64,
    /// Where the next body goes.
    pub body_end: u64,
    /// Where the next refused-row record goes (a record index).
    pub refusals_end: u32,
}

impl Segment {
    fn count(&self) -> u32 {
        u32::try_from(self.receipts.len()).unwrap_or(u32::MAX)
    }

    /// The receipt that holds a body with this digest, by kind (C-RK-01's
    /// bound: at most [`MAX_RECEIPTS`] receipts).
    fn holding(&self, digest: &[u8; 32], kind: Kept) -> Option<(u32, Receipt)> {
        let found = self
            .receipts
            .iter()
            .position(|r| r.kept == kind && r.digest == *digest)?;
        let receipt = *self.receipts.get(found)?;
        Some((u32::try_from(found).ok()?, receipt))
    }
}

/// Reads one segment's receipts under the torn-tail rule.
///
/// # Errors
///
/// [`KeepError::Damaged`] when a receipt before the last fails its CRC, or
/// the file is longer than [`MAX_RECEIPTS`] receipts and a torn one.
pub fn read_segment(names: &Names) -> Result<Segment, KeepError> {
    let Some(file) = open_regular(&names.receipts, false)? else {
        return Ok(Segment::default());
    };
    let cap = u64::from(MAX_RECEIPTS) * RECEIPT_LEN as u64 + RECEIPT_LEN as u64 - 1;
    let bytes = read_bounded(&file, &names.receipts, cap)?;
    let whole = bytes.len() / RECEIPT_LEN;
    let mut segment = Segment::default();
    for (index, chunk) in bytes.chunks_exact(RECEIPT_LEN).enumerate() {
        match Receipt::decode(chunk) {
            Some(receipt) => {
                // A CRC-VALID RECEIPT IS STILL ONLY A CLAIM ABOUT LENGTHS. No
                // writer keeps a body that ends past `MAX_PACK_BYTES` or a
                // record past `REFUSAL_ROOM`, so one that names either is
                // damage, refused before its sum can overflow and send the
                // next append over records already kept (repair round 1).
                let body_end = receipt
                    .body_offset
                    .checked_add(u64::from(receipt.deflated_len));
                let rows_end = u64::from(receipt.refusals_from) + u64::from(receipt.refusals);
                let Some(body_end) = body_end.filter(|end| *end <= MAX_PACK_BYTES) else {
                    return Err(KeepError::Damaged {
                        path: names.receipts.clone(),
                        ordinal: index as u64,
                    });
                };
                if rows_end > REFUSAL_ROOM {
                    return Err(KeepError::Damaged {
                        path: names.receipts.clone(),
                        ordinal: index as u64,
                    });
                }
                if matches!(receipt.kept, Kept::Kept | Kept::RefusedHead) {
                    segment.body_end = body_end;
                }
                if receipt.kept != Kept::InFirstMonth && receipt.refusals > 0 {
                    // At most `REFUSAL_ROOM`, checked above: it fits a u32.
                    segment.refusals_end = receipt.refusals_from + receipt.refusals;
                }
                segment.receipts.push(receipt);
            }
            // THE LAST WHOLE RECEIPT MAY BE TORN: a crash after the length
            // grew and before the bytes reached the disk. Anything earlier
            // was synced before a later receipt was written, so a bad CRC
            // there is damage and is refused, never overwritten.
            None if index + 1 == whole => {}
            None => {
                return Err(KeepError::Damaged {
                    path: names.receipts.clone(),
                    ordinal: index as u64,
                });
            }
        }
    }
    let trusted = segment.receipts.len() * RECEIPT_LEN;
    segment.torn_receipt_bytes = (bytes.len() - trusted) as u64;
    Ok(segment)
}

/// The newest segment of a month that exists (0 when none does): at most
/// [`MAX_SEGMENTS`] `stat`s.
fn newest_segment(dir: &Path, month: YearMonth) -> u8 {
    let mut newest = 0;
    for segment in 1..MAX_SEGMENTS {
        if names(dir, month, segment).receipts.exists() {
            newest = segment;
        } else {
            break;
        }
    }
    newest
}

/// Every trusted receipt of a month, across its segments.
///
/// # Errors
///
/// As [`read_segment`].
pub fn receipts(
    root: &Path,
    pack: Pack<'_>,
    month: YearMonth,
) -> Result<Vec<(u8, Segment)>, KeepError> {
    let dir = pack_dir(root, pack)?;
    let newest = newest_segment(&dir, month);
    (0..=newest)
        .map(|segment| Ok((segment, read_segment(&names(&dir, month, segment))?)))
        .collect()
}

/// The body a receipt names, inflated and checked against its length and
/// BLAKE3.
///
/// # Errors
///
/// [`KeepError::NotKept`] for `kept=0`; [`KeepError::BodyMismatch`] when
/// the bytes do not match the receipt.
pub fn body(
    root: &Path,
    pack: Pack<'_>,
    month: YearMonth,
    receipt: &Receipt,
) -> Result<String, KeepError> {
    if receipt.kept == Kept::NotKept {
        return Err(KeepError::NotKept {
            reason: receipt.reason,
        });
    }
    let home = if receipt.kept == Kept::InFirstMonth {
        month_of_unix(receipt.asked_from)?
    } else {
        month
    };
    let dir = pack_dir(root, pack)?;
    let path = names(&dir, home, receipt.segment).bodies;
    let Some(file) = open_regular(&path, false)? else {
        return Err(KeepError::BodyMismatch {
            path,
            what: "is missing",
        });
    };
    // A CRC-VALID RECEIPT IS STILL ONLY A CLAIM ABOUT LENGTHS. No writer
    // keeps a body that ends past a segment's cap or inflates past the
    // largest answer, so either is damage, refused before a byte is
    // allocated (up to 4 GiB a receipt could otherwise ask for).
    if receipt
        .body_offset
        .saturating_add(u64::from(receipt.deflated_len))
        > MAX_PACK_BYTES
        || receipt.raw_len as usize > crate::http::MAX_RESPONSE_BYTES
    {
        return Err(KeepError::BodyMismatch {
            path,
            what: "names a length past the pack's caps",
        });
    }
    let mut deflated = vec![0_u8; receipt.deflated_len as usize];
    file.read_exact_at(&mut deflated, receipt.body_offset)
        .map_err(|why| KeepError::io("read", &path, &why))?;
    let mut raw = Vec::with_capacity(receipt.raw_len as usize);
    let read = flate2::read::DeflateDecoder::new(deflated.as_slice())
        .take(u64::from(receipt.raw_len) + 1)
        .read_to_end(&mut raw);
    if read.is_err() || raw.len() != receipt.raw_len as usize {
        return Err(KeepError::BodyMismatch {
            path,
            what: "does not inflate to its receipt's length",
        });
    }
    if brutex_core::blake3::hash(&raw) != receipt.digest {
        return Err(KeepError::BodyMismatch {
            path,
            what: "does not match its receipt's BLAKE3",
        });
    }
    String::from_utf8(raw).map_err(|_| KeepError::BodyMismatch {
        path,
        what: "is not UTF-8",
    })
}

/// The refused rows a receipt owns (DB-G46). `ordinal` is the receipt's
/// own place in its segment; a `kept=3` receipt's rows are its first
/// month's receipt's, `body_of`.
///
/// # Errors
///
/// [`KeepError::Damaged`] when a record fails its CRC or names another
/// receipt.
pub fn refused_rows(
    root: &Path,
    pack: Pack<'_>,
    month: YearMonth,
    ordinal: u32,
    receipt: &Receipt,
) -> Result<Vec<RefusedRow>, KeepError> {
    if receipt.refusals == 0 {
        return Ok(Vec::new());
    }
    let home = if receipt.kept == Kept::InFirstMonth {
        month_of_unix(receipt.asked_from)?
    } else {
        month
    };
    let path = names(&pack_dir(root, pack)?, home, receipt.segment).refusals;
    // NO WRITER PUTS A RECORD PAST [`REFUSAL_ROOM`], so a receipt that names
    // one is damage, refused before up to 96 GiB is allocated for it.
    if u64::from(receipt.refusals_from) + u64::from(receipt.refusals) > REFUSAL_ROOM {
        return Err(KeepError::Damaged {
            path,
            ordinal: u64::from(receipt.refusals_from),
        });
    }
    let Some(file) = open_regular(&path, false)? else {
        return Err(KeepError::Damaged { path, ordinal: 0 });
    };
    let owner = if receipt.kept == Kept::InFirstMonth {
        receipt.body_of
    } else {
        ordinal
    };
    let mut bytes = vec![0_u8; receipt.refusals as usize * REFUSAL_LEN];
    file.read_exact_at(
        &mut bytes,
        u64::from(receipt.refusals_from) * REFUSAL_LEN as u64,
    )
    .map_err(|why| KeepError::io("read", &path, &why))?;
    bytes
        .chunks_exact(REFUSAL_LEN)
        .enumerate()
        .map(|(i, chunk)| {
            RefusedRow::decode(chunk)
                .filter(|(_, named)| *named == owner)
                .map(|(row, _)| row)
                .ok_or_else(|| KeepError::Damaged {
                    path: path.clone(),
                    ordinal: u64::from(receipt.refusals_from) + i as u64,
                })
        })
        .collect()
}

/// Holds a month's pack lock for its lifetime.
#[derive(Debug)]
struct Held {
    _lock: store::flock::Flock<File, PathBuf>,
}

fn lock(dir: &Path) -> Result<Held, KeepError> {
    lock_within(dir, LOCK_WAIT)
}

fn lock_within(dir: &Path, wait: Duration) -> Result<Held, KeepError> {
    let path = lock_path(dir);
    let open = || match open_regular(&path, true) {
        Ok(Some(file)) => Ok(file),
        Ok(None) => Err(KeepError::NotRegular { path: path.clone() }),
        Err(why) => Err(why),
    };
    match store::flock::Flock::try_lock(open()?, path.clone()) {
        Ok(held) => return Ok(Held { _lock: held }),
        Err(std::fs::TryLockError::WouldBlock) => {}
        Err(std::fs::TryLockError::Error(why)) => return Err(KeepError::io("lock", &path, &why)),
    }
    // CONTENDED: A BLOCKING WAIT, NOT A POLL. A `try_lock` every 10 ms
    // starved: a landing that re-takes the lock a few microseconds after it
    // lets go wins every race against a poller that looks once per 10 ms, so
    // under load the second landing of a pack was refused `lock_stalled`
    // after 30 s while the first never paused (measured: 2,128 tries, in the
    // full `pull` suite on 14 Oct 2026). A waiter blocked in `flock` is woken
    // when the holder lets go. The wait runs on one named thread so it can be
    // bounded; past `wait` the caller is refused, and the waiter, once it
    // gets the lock, finds nobody listening and lets it go at once.
    //
    // AN ABANDONED WAITER STAYS PARKED until the holder lets go, so a holder
    // that never does would leave one thread and one descriptor behind per
    // refused append. At most `MAX_PARKED_WAITERS` ABANDONED waiters are
    // counted per lock; the next contended append is refused at once, by
    // name, and waits for nothing. A waiter whose caller is still inside its
    // wait is NOT counted (repair round 2 of IA-8: counting every waiter
    // refused the sixth of six healthy concurrent appends of one pack at
    // once, with a false "holder that has not let go").
    Parked::admit(&path)?;
    let file = open()?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let abandoned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let waiter = Waiter {
        path: path.clone(),
        abandoned: std::sync::Arc::clone(&abandoned),
    };
    std::thread::Builder::new()
        .name("keep-lock".to_owned())
        .spawn(move || {
            let taken = store::flock::Flock::lock(file, waiter.path.clone());
            waiter.answer(taken, &tx);
        })
        .map_err(|why| KeepError::io("lock", &path, &why))?;
    let started = Instant::now();
    let answer = match rx.recv_timeout(wait) {
        Ok(answer) => answer,
        Err(_) => match Parked::abandon(&path, &abandoned, &rx) {
            // THE WAITER ANSWERED between the timeout and the abandon: the
            // lock is ours, and nothing is parked.
            Some(answer) => answer,
            None => {
                return Err(KeepError::LockStalled {
                    lock: path,
                    waited_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                });
            }
        },
    };
    match answer {
        Ok(held) => Ok(Held { _lock: held }),
        Err(why) => Err(KeepError::io("lock", &path, &why)),
    }
}

/// What a waiter thread hands back: the lock, or why `flock` failed.
type Taken = std::io::Result<store::flock::Flock<File, PathBuf>>;

/// The waiter thread's half of one contended wait.
#[derive(Debug)]
struct Waiter {
    path: PathBuf,
    /// Set, under [`PARKED`], by a caller whose wait ran out.
    abandoned: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Waiter {
    /// Hands the lock to its caller, or, when the caller has already
    /// abandoned the wait, lets it go and leaves the parked count. Decided
    /// under [`PARKED`], the lock the caller abandons under, so the two
    /// cannot both think the other has the lock.
    fn answer(self, taken: Taken, tx: &std::sync::mpsc::SyncSender<Taken>) {
        let mut parked = PARKED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.abandoned.load(std::sync::atomic::Ordering::Relaxed) {
            let left = parked.get(&self.path).map_or(0, |n| n.saturating_sub(1));
            if left == 0 {
                parked.remove(&self.path);
            } else {
                parked.insert(self.path, left);
            }
            drop(parked);
            drop(taken);
        } else {
            // A one-slot channel nobody has written to: this never blocks.
            // The caller is still listening, or about to look under PARKED.
            drop(tx.send(taken));
        }
    }
}

/// Waiter threads whose caller abandoned the wait and that are still parked
/// in `flock` on each contended pack lock, by lock path; a path leaves the
/// map when its last abandoned waiter takes the lock and lets it go.
/// Pre-sized for the pack locks one process contends at once (its landings in
/// flight); a larger count only grows the map.
static PARKED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<PathBuf, usize>>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(std::collections::HashMap::with_capacity(PARKED_LOCKS))
    });

/// The pack locks [`PARKED`] is sized for.
const PARKED_LOCKS: usize = 16;

/// The abandoned-waiter count of [`PARKED`]: one hash probe under one mutex
/// per contended wait.
#[derive(Debug)]
struct Parked;

impl Parked {
    /// Refuses a new wait once `MAX_PARKED_WAITERS` abandoned waiters are
    /// parked on `lock`; a live wait is never counted.
    fn admit(lock: &Path) -> Result<(), KeepError> {
        let parked = PARKED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(lock)
            .copied()
            .unwrap_or(0);
        if parked >= MAX_PARKED_WAITERS {
            return Err(KeepError::LockWaitersParked {
                lock: lock.to_path_buf(),
                parked,
            });
        }
        Ok(())
    }

    /// The caller's wait ran out: unless the waiter answered meanwhile, it
    /// is marked abandoned and counted, under the same lock the waiter
    /// decides under.
    fn abandon(
        lock: &Path,
        abandoned: &std::sync::atomic::AtomicBool,
        rx: &std::sync::mpsc::Receiver<Taken>,
    ) -> Option<Taken> {
        let mut parked = PARKED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(answer) = rx.try_recv() {
            return Some(answer);
        }
        abandoned.store(true, std::sync::atomic::Ordering::Relaxed);
        *parked.entry(lock.to_path_buf()).or_insert(0) += 1;
        None
    }
}

/// Test-only: how many abandoned waiters are parked on a lock now.
#[cfg(test)]
fn parked_on(lock: &Path) -> usize {
    PARKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(lock)
        .copied()
        .unwrap_or(0)
}

/// Writes `bytes` at `at`, cuts the file there (a torn tail past it goes),
/// and syncs. A write at offset 0 is a file's first trusted bytes, whether the
/// file was born here or holds only a torn tail an interrupted append left:
/// its directory is synced too, so the name survives a power loss with its
/// bytes. One rule for every pack file, a `.rsb` or `.rsx` born after its
/// month's `.rsr` included (verify r2 follow-up).
fn write_synced(path: &Path, at: u64, bytes: &[u8]) -> Result<(), KeepError> {
    let Some(file) = open_regular(path, true)? else {
        return Err(KeepError::NotRegular {
            path: path.to_path_buf(),
        });
    };
    #[cfg(test)]
    if FAULT.with(std::cell::Cell::get) == Some(path_kind(path))
        && FAULT_SKIP.with(|skip| skip.replace(skip.get().saturating_sub(1))) == 0
    {
        return Err(KeepError::Io {
            op: "write",
            path: path.to_path_buf(),
            kind: io::ErrorKind::StorageFull,
        });
    }
    file.write_all_at(bytes, at)
        .and_then(|()| file.set_len(at + bytes.len() as u64))
        .and_then(|()| file.sync_data())
        .map_err(|why| KeepError::io("write", path, &why))?;
    if at == 0 {
        sync_parent(path)?;
    }
    Ok(())
}

/// Syncs the directory holding `path`, so an entry born in it is durable.
fn sync_parent(path: &Path) -> Result<(), KeepError> {
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    #[cfg(test)]
    DIR_SYNCS.with(|synced| synced.borrow_mut().push(dir.to_path_buf()));
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|why| KeepError::io("sync the directory", dir, &why))
}

/// Creates the pack directory and every missing parent, syncing the parent
/// of each one it creates, so a power loss cannot take the directory a synced
/// pack file was written into (verify r2 follow-up). At most the path's
/// depth of `lstat`s.
fn create_dir_synced(dir: &Path) -> Result<(), KeepError> {
    let mut missing = Vec::new();
    let mut at = Some(dir);
    while let Some(one) = at {
        if fs::symlink_metadata(one).is_ok() {
            break;
        }
        missing.push(one);
        at = one.parent();
    }
    fs::create_dir_all(dir).map_err(|why| KeepError::io("create", dir, &why))?;
    for born in missing.iter().rev() {
        sync_parent(born)?;
    }
    Ok(())
}

#[cfg(test)]
fn path_kind(path: &Path) -> u8 {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rsb") => 1,
        Some("rsx") => 2,
        Some("rsw") => 4,
        _ => 3,
    }
}

#[cfg(test)]
thread_local! {
    /// Test-only: refuse the next writes to one kind of file as a full disk.
    pub(crate) static FAULT: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) };
    /// Test-only: how many writes of the `FAULT` kind go through before it
    /// refuses. A refused write stands for a kill at that point: nothing
    /// after it is written (repair round 3).
    pub(crate) static FAULT_SKIP: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Test-only: every directory synced on this thread, in order.
    pub(crate) static DIR_SYNCS: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn deflate(raw: &[u8]) -> Result<Vec<u8>, KeepError> {
    let mut encoder = flate2::write::DeflateEncoder::new(
        Vec::with_capacity(raw.len() / 4),
        flate2::Compression::default(),
    );
    encoder
        .write_all(raw)
        .and_then(|()| encoder.finish())
        .map_err(|why| KeepError::io("deflate", Path::new(KEEP_ROOT), &why))
}

/// Keeps one answer: its body in the window's first month's pack, a receipt
/// there, and a receipt in every further month the window covers.
///
/// Called AFTER the landing returns (land, keep, spool: D-4859), through
/// either door. Every refusal is also an audited error event "response not
/// kept" that names the pack (§7.1).
///
/// # Errors
///
/// Any [`KeepError`]; what was written before it stays and is true.
pub fn append(
    root: &Path,
    pack: Pack<'_>,
    offered: Offered<'_>,
    landing: &Landing<'_>,
) -> Result<Outcome, KeepError> {
    let result = append_unlogged(root, pack, offered, landing);
    match &result {
        Ok(outcome) => note_kept(root, pack, outcome),
        Err(why) => note_not_kept(root, pack, landing, why),
    }
    result
}

fn note_kept(root: &Path, pack: Pack<'_>, outcome: &Outcome) {
    let key = pack.key.to_string();
    let first = &outcome.first;
    // EVERY ROLL IS NAMED, the first month's and each further month's: a
    // window over months whose later month rolled used to roll in silence,
    // because only `first` was looked at (repair round 2 of IA-8).
    for placed in std::iter::once(first).chain(&outcome.further) {
        if placed.rolled {
            note_rolled(root, pack, &key, placed);
        }
    }
    if first.receipt.kept == Kept::NotKept {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error("pull.keep", "response not kept")
                .with("instrument", telemetry::Value::Str(&key))
                .with("timeframe", telemetry::Value::Str(pack.timeframe.as_str()))
                .with("reason", telemetry::Value::Str(first.receipt.reason.word())),
        );
    }
}

/// One roll: the month, the segment it rolled to and that segment's receipts
/// file, so the event names the pack it is about.
fn note_rolled(root: &Path, pack: Pack<'_>, key: &str, placed: &Placed) {
    let month = placed.month.to_string();
    let file = pack_dir(root, pack).map_or_else(
        |_| key.to_owned(),
        |dir| {
            names(&dir, placed.month, placed.segment)
                .receipts
                .display()
                .to_string()
        },
    );
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(
            "pull.keep",
            "response pack segment full; keeping goes on in the next segment",
        )
        .with("instrument", telemetry::Value::Str(key))
        .with("timeframe", telemetry::Value::Str(pack.timeframe.as_str()))
        .with("month", telemetry::Value::Str(&month))
        .with("segment", telemetry::Value::Uint(u64::from(placed.segment)))
        .with("pack", telemetry::Value::Str(&file)),
    );
}

fn note_not_kept(root: &Path, pack: Pack<'_>, landing: &Landing<'_>, why: &KeepError) {
    let key = pack.key.to_string();
    let dir = pack_dir(root, pack).map_or_else(|_| key.clone(), |d| d.display().to_string());
    let text = why.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.keep", "response not kept")
            .with("pack", telemetry::Value::Str(&dir))
            .with("reason", telemetry::Value::Str(why.word()))
            .with("detail", telemetry::Value::Str(&text))
            // THE ROWS THIS REFUSAL LEFT UNKEPT, counted where nothing on
            // disk does: no receipt was written for them.
            .with(
                "refused_rows",
                telemetry::Value::Uint(landing.refused.len() as u64),
            ),
    );
}

/// The receipt every month of one append starts from: the landing's counts
/// and identity, the offered bytes' lengths and BLAKE3.
fn template_of(
    pack: Pack<'_>,
    landing: &Landing<'_>,
    (raw, kind, status, whole_len): (&[u8], Kept, u16, i64),
    deflated: &[u8],
) -> Receipt {
    let raw_len = raw.len() as u64;
    let (asked_from, asked_to) = asked_span(landing.window);
    let refused_count = u32::try_from(landing.refused.len()).unwrap_or(u32::MAX);
    Receipt {
        asked_from,
        asked_to,
        first_stamp: if kind == Kept::RefusedHead {
            whole_len
        } else {
            landing.first_stamp
        },
        last_stamp: landing.last_stamp,
        body_offset: 0,
        deflated_len: u32::try_from(deflated.len()).unwrap_or(u32::MAX),
        raw_len: u32::try_from(raw_len).unwrap_or(u32::MAX),
        rows_decoded: landing.rows_decoded,
        rows_landed: landing.rows_landed,
        rows_refused: landing.rows_refused.max(refused_count),
        run: landing.run,
        status,
        rung: landing.rung,
        kept: kind,
        reason: if kind == Kept::RefusedHead {
            Reason::RefusedAnswer
        } else {
            Reason::Kept
        },
        rules_version: landing.rules_version,
        door: landing.door,
        segment: 0,
        digest: brutex_core::blake3::hash(raw),
        key_hash: store::verdict::key_hash(pack.key.to_string().as_bytes()),
        body_of: 0,
        refusals_from: 0,
        refusals: 0,
    }
}

fn append_unlogged(
    root: &Path,
    pack: Pack<'_>,
    offered: Offered<'_>,
    landing: &Landing<'_>,
) -> Result<Outcome, KeepError> {
    let dir = pack_dir(root, pack)?;
    let months = months_of(landing.window)?;
    let (raw, kind, status, whole_len) = match offered {
        Offered::Answer(answer) => (answer.body.as_bytes(), Kept::Kept, answer.status, 0),
        Offered::Refused(refused) => (
            refused.head.as_bytes(),
            Kept::RefusedHead,
            refused.status,
            i64::try_from(refused.len).unwrap_or(i64::MAX),
        ),
    };
    let raw_len = raw.len() as u64;
    if raw_len > crate::http::MAX_RESPONSE_BYTES as u64 {
        return Err(KeepError::TooLarge { len: raw_len });
    }
    let deflated = deflate(raw)?;
    create_dir_synced(&dir)?;
    let template = template_of(pack, landing, (raw, kind, status, whole_len), &deflated);
    let mut months = months.into_iter();
    let first_month = months
        .next()
        .ok_or(KeepError::Path(PathError::EmptySegment { field: "month" }))?;
    let _held = lock(&dir)?;
    // AN INTERRUPTED WINDOW OVER MONTHS IS FINISHED BEFORE ANYTHING ELSE, under
    // the same lock: a writer killed between its first month's receipt and a
    // later month's pointer left that receipt with no pointer, for good, and
    // the next append carried on as if the pack were whole (verify r3). Every
    // append looks, so nothing has been appended since the interrupted one.
    let journal = dir.join(WINDOWS_NAME);
    let next_record = finish_window(root, pack, &dir, &journal)?;
    // EVERY LATER MONTH IS ASKED FIRST whether it can take its pointer: one
    // whose last segment holds `MAX_RECEIPTS` receipts used to be refused
    // `receipts_full` ("nothing written") only after the first month's body,
    // rows and receipt were already kept (repair round 1). The lock is the
    // pack's, so nothing changes between this look and the writes, and the
    // segment each look chose is the one its pointer is written to.
    let chosen = months
        .map(|month| Ok((month, choose_segment(&dir, month, 0)?)))
        .collect::<Result<Vec<_>, KeepError>>()?;
    let over_months = !chosen.is_empty();
    if over_months && next_record + 2 > MAX_WINDOW_RECORDS {
        return Err(KeepError::WindowsFull { journal });
    }
    let record = |segment: u8, ordinal: u32, state: WindowState| WindowRecord {
        asked_from: template.asked_from,
        asked_to: template.asked_to,
        digest: template.digest,
        run: template.run,
        month: first_month,
        segment,
        ordinal: u16::try_from(ordinal).unwrap_or(u16::MAX),
        state,
        written: 0,
    };
    // THE OPEN RECORD IS WRITTEN AFTER THE FIRST MONTH HAS CHOSEN WHERE ITS
    // RECEIPT GOES AND BEFORE ITS FIRST BYTE: a refusal before that point
    // still writes nothing, and the record names the exact place a finishing
    // append looks for the receipt.
    let mut open = |segment: u8, ordinal: u32| -> Result<(), KeepError> {
        if over_months {
            write_window(
                &journal,
                next_record,
                &record(segment, ordinal, WindowState::Open),
            )?;
        }
        Ok(())
    };
    let first = place_body(
        &dir,
        first_month,
        &deflated,
        template,
        landing.refused,
        &mut open,
    )?;
    let mut further = Vec::with_capacity(chosen.len());
    for (month, segment) in chosen {
        further.push(place_receipt_only(month, segment, pointer_to(&first))?);
    }
    // THE CLOSING RECORD IS THE WINDOW'S COMMIT: until it is on disk, the
    // next append finishes whatever pointer is missing.
    if over_months {
        write_window(
            &journal,
            next_record + 1,
            &record(first.segment, first.ordinal, WindowState::Closed),
        )?;
    }
    Ok(Outcome { first, further })
}

/// The pointer a further month gets to the first month's receipt: the one
/// rule the append and the finishing of an interrupted window both use.
fn pointer_to(first: &Placed) -> Receipt {
    let mut pointer = first.receipt;
    if pointer.kept == Kept::NotKept {
        // NO BODY TO POINT AT: the reason stands, and the refused rows
        // are read from the first month's receipt: its month from
        // `asked_from`, its segment and ordinal from `segment` and
        // `body_of`, which the first receipt already carries.
        pointer.refusals_from = 0;
        pointer.refusals = 0;
    } else {
        pointer.kept = Kept::InFirstMonth;
        pointer.reason = Reason::LaterMonth;
        pointer.body_of = first.ordinal;
    }
    pointer
}

/// The window journal's trusted tail: the index the next record is written
/// at, and the last record a reader may trust. One `stat` and at most two
/// 64-byte reads, whatever the journal's length.
///
/// # Errors
///
/// [`KeepError::Damaged`] when the record before a torn last one fails its
/// CRC too, or the journal is longer than [`MAX_WINDOW_RECORDS`] records.
fn read_windows(path: &Path) -> Result<(u64, Option<WindowRecord>), KeepError> {
    let Some(file) = open_regular(path, false)? else {
        return Ok((0, None));
    };
    let len = file
        .metadata()
        .map_err(|why| KeepError::io("stat", path, &why))?
        .len();
    let whole = len / WINDOW_RECORD_LEN as u64;
    if whole > MAX_WINDOW_RECORDS {
        return Err(KeepError::Damaged {
            path: path.to_path_buf(),
            ordinal: MAX_WINDOW_RECORDS,
        });
    }
    let read_at = |index: u64| {
        let mut bytes = [0_u8; WINDOW_RECORD_LEN];
        file.read_exact_at(&mut bytes, index * WINDOW_RECORD_LEN as u64)
            .map_err(|why| KeepError::io("read", path, &why))?;
        Ok::<_, KeepError>(WindowRecord::decode(&bytes))
    };
    let Some(last) = whole.checked_sub(1) else {
        return Ok((0, None));
    };
    if let Some(record) = read_at(last)? {
        return Ok((whole, Some(record)));
    }
    // THE LAST WHOLE RECORD MAY BE TORN, as the last receipt may: the next
    // record is written over it, and the one before it was synced first.
    let Some(before) = last.checked_sub(1) else {
        return Ok((0, None));
    };
    match read_at(before)? {
        Some(record) => Ok((last, Some(record))),
        None => Err(KeepError::Damaged {
            path: path.to_path_buf(),
            ordinal: before,
        }),
    }
}

fn write_window(path: &Path, index: u64, record: &WindowRecord) -> Result<(), KeepError> {
    write_synced(path, index * WINDOW_RECORD_LEN as u64, &record.encode())
}

/// The pack's window over months that an interrupted append left open, if
/// any: its first month, the segment and ordinal of that month's receipt,
/// and its asked span. While no writer holds the pack lock, `Some` means a
/// first month's receipt may stand with a later month missing its pointer;
/// the next append under the lock finishes it and says so (repair round 3).
///
/// # Errors
///
/// As [`pack_dir`], and [`KeepError::Damaged`] for a damaged journal.
pub fn unfinished(root: &Path, pack: Pack<'_>) -> Result<Option<WindowRecord>, KeepError> {
    let dir = pack_dir(root, pack)?;
    let (_next, last) = read_windows(&dir.join(WINDOWS_NAME))?;
    Ok(last.filter(|record| record.state == WindowState::Open))
}

/// Finishes the window over months an interrupted append left open, under
/// the pack lock, and returns where the journal's next record goes.
///
/// The open record names where the first month's receipt was to go. Nothing
/// has been appended since (every append comes through here first), so that
/// segment holds exactly `ordinal` receipts when the receipt was never
/// written, and the receipt the record names at `ordinal` when it was. In
/// the second case every further month whose newest receipt is not the
/// pointer to it gets that pointer now, exactly as the interrupted append
/// would have written it. Either way a closing record is written and an
/// audited error event names the window, so the interruption is never
/// silent.
///
/// # Errors
///
/// Any refusal of the writes, and [`KeepError::Damaged`] when the first
/// month does not hold what the open record says it can.
fn finish_window(
    root: &Path,
    pack: Pack<'_>,
    dir: &Path,
    journal: &Path,
) -> Result<u64, KeepError> {
    let (next, last) = read_windows(journal)?;
    let Some(open) = last.filter(|record| record.state == WindowState::Open) else {
        return Ok(next);
    };
    let damaged = || KeepError::Damaged {
        path: journal.to_path_buf(),
        ordinal: next.saturating_sub(1),
    };
    let at = names(dir, open.month, open.segment);
    let state = read_segment(&at)?;
    let ordinal = u32::from(open.ordinal);
    let mut placed = Vec::new();
    let finished = if state.count() == ordinal {
        WindowState::Abandoned
    } else {
        let receipt = state
            .receipts
            .get(usize::from(open.ordinal))
            .copied()
            .filter(|receipt| open.names(receipt))
            .ok_or_else(damaged)?;
        let first = Placed {
            month: open.month,
            segment: open.segment,
            ordinal,
            receipt,
            rolled: false,
        };
        let pointer = pointer_to(&first);
        let months = months_from(open.month, month_of_unix(open.asked_to)?)?;
        for month in months.into_iter().skip(1) {
            let index = newest_segment(dir, month);
            let names_of = names(dir, month, index);
            let held = read_segment(&names_of)?;
            if held.receipts.last() == Some(&pointer) {
                continue;
            }
            let chosen = roll_if_full(dir, month, 0, 0, (index, names_of, held))?;
            placed.push(place_receipt_only(month, chosen, pointer)?);
        }
        WindowState::Completed
    };
    let closing = WindowRecord {
        state: finished,
        written: u8::try_from(placed.len()).unwrap_or(u8::MAX),
        ..open
    };
    write_window(journal, next, &closing)?;
    note_finished(root, pack, &closing, &placed);
    Ok(next + 1)
}

/// The audited event for a window an interrupted append left open, and one
/// roll event per pointer it wrote into a new segment.
fn note_finished(root: &Path, pack: Pack<'_>, closing: &WindowRecord, placed: &[Placed]) {
    let key = pack.key.to_string();
    let month = closing.month.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(
            "pull.keep",
            "an interrupted window over months was finished by the next append",
        )
        .with("instrument", telemetry::Value::Str(&key))
        .with("timeframe", telemetry::Value::Str(pack.timeframe.as_str()))
        .with("month", telemetry::Value::Str(&month))
        .with(
            "segment",
            telemetry::Value::Uint(u64::from(closing.segment)),
        )
        .with(
            "ordinal",
            telemetry::Value::Uint(u64::from(closing.ordinal)),
        )
        .with("state", telemetry::Value::Str(closing.state.word()))
        .with(
            "pointers_written",
            telemetry::Value::Uint(placed.len() as u64),
        ),
    );
    for one in placed {
        if one.rolled {
            note_rolled(root, pack, &key, one);
        }
    }
}

/// The month's newest segment, rolled forward when it cannot take one more
/// receipt or a body of `deflated` bytes and a later segment can.
fn choose_segment(
    dir: &Path,
    month: YearMonth,
    deflated: u64,
) -> Result<(u8, Names, Segment, bool), KeepError> {
    let index = newest_segment(dir, month);
    let at = names(dir, month, index);
    let state = read_segment(&at)?;
    roll_if_full(dir, month, deflated, 0, (index, at, state))
}

/// Whether a segment whose `.rsx` holds `held` records can take `rows` more.
fn rows_fit(held: u32, rows: usize) -> bool {
    rows == 0 || u64::from(held) + rows as u64 <= REFUSAL_ROOM
}

/// The newest segment as read, or the next one when the newest cannot take
/// one more receipt, a body of `deflated` bytes or `rows` refused-row
/// records and a later segment can.
///
/// # Errors
///
/// [`KeepError::ReceiptsFull`] and [`KeepError::RefusalsFull`] when no
/// segment can take them: nothing has been written yet.
fn roll_if_full(
    dir: &Path,
    month: YearMonth,
    deflated: u64,
    rows: usize,
    (index, at, state): (u8, Names, Segment),
) -> Result<(u8, Names, Segment, bool), KeepError> {
    // MORE ROWS THAN ANY SEGMENT HOLDS are refused before anything else is
    // weighed: a roll taken for a full receipts file or a full body file
    // would otherwise hand them to an empty segment that cannot take them
    // either, and they would be written past its room (repair round 1).
    if !rows_fit(0, rows) {
        return Err(KeepError::RefusalsFull {
            pack: at.refusals,
            held: state.refusals_end,
            rows: rows as u64,
        });
    }
    let receipts_full = state.count() >= MAX_RECEIPTS;
    let body_full = state.body_end + deflated > MAX_PACK_BYTES;
    let fits_empty = deflated <= MAX_PACK_BYTES;
    // THE REFUSED ROWS ARE A CAP LIKE THE BODIES (DB-G46, verify round 1):
    // they used to be skipped, with the receipt still written and `Ok`
    // returned, once the `.rsx` was full, and nothing rolled on them.
    let rows_full = !rows_fit(state.refusals_end, rows);
    if (receipts_full || (body_full && fits_empty) || rows_full) && index + 1 < MAX_SEGMENTS {
        let next = index + 1;
        return Ok((next, names(dir, month, next), Segment::default(), true));
    }
    if receipts_full {
        return Err(KeepError::ReceiptsFull { pack: at.receipts });
    }
    if rows_full {
        return Err(KeepError::RefusalsFull {
            pack: at.refusals,
            held: state.refusals_end,
            rows: rows as u64,
        });
    }
    Ok((index, at, state, false))
}

/// Keeps the first month's body, rows and receipt. `open` is called once, with
/// the segment and ordinal the receipt goes to, after every refusal that writes
/// nothing and before the first byte is written.
fn place_body(
    dir: &Path,
    month: YearMonth,
    deflated: &[u8],
    template: Receipt,
    refused: &[RefusedRow],
    open: &mut dyn FnMut(u8, u32) -> Result<(), KeepError>,
) -> Result<Placed, KeepError> {
    let index = newest_segment(dir, month);
    let newest = names(dir, month, index);
    let state = read_segment(&newest)?;
    // THE DUPLICATE CHECK COMES BEFORE ANY ROLL: a repeated body whose
    // segment has room for one more receipt and for its refused rows is a
    // receipt there, even when that segment could not take the body a second
    // time. Without room for the rows it goes through the roll like any body.
    if state.count() < MAX_RECEIPTS
        && rows_fit(state.refusals_end, refused.len())
        && let Some((holder, held)) = state.holding(&template.digest, template.kept)
    {
        let mut receipt = template;
        receipt.kept = Kept::Duplicate;
        receipt.reason = Reason::DuplicateBody;
        receipt.segment = index;
        receipt.body_of = holder;
        receipt.body_offset = held.body_offset;
        receipt.deflated_len = held.deflated_len;
        let ordinal = state.count();
        open(index, ordinal)?;
        return write_refusals_then_receipt(
            &newest, &state, ordinal, receipt, refused, index, month, false,
        );
    }
    let (segment, at, state, rolled) = roll_if_full(
        dir,
        month,
        deflated.len() as u64,
        refused.len(),
        (index, newest, state),
    )?;
    let ordinal = state.count();
    let mut receipt = template;
    receipt.segment = segment;
    receipt.body_of = ordinal;
    open(segment, ordinal)?;
    if state.body_end + deflated.len() as u64 > MAX_PACK_BYTES {
        receipt.kept = Kept::NotKept;
        receipt.reason = Reason::PackFull;
        receipt.deflated_len = 0;
    } else {
        receipt.body_offset = state.body_end;
        if let Err(why) = write_synced(&at.bodies, state.body_end, deflated) {
            if !matches!(
                why,
                KeepError::Io {
                    kind: io::ErrorKind::StorageFull,
                    ..
                }
            ) {
                return Err(why);
            }
            receipt.kept = Kept::NotKept;
            receipt.reason = Reason::StorageFull;
            receipt.body_offset = 0;
            receipt.deflated_len = 0;
        }
    }
    write_refusals_then_receipt(
        &at, &state, ordinal, receipt, refused, segment, month, rolled,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "one private step of `place_body`, every argument a value it already holds"
)]
fn write_refusals_then_receipt(
    at: &Names,
    state: &Segment,
    ordinal: u32,
    mut receipt: Receipt,
    refused: &[RefusedRow],
    segment: u8,
    month: YearMonth,
    rolled: bool,
) -> Result<Placed, KeepError> {
    // EVERY CALLER HAS ALREADY MADE ROOM: the duplicate path only where the
    // rows fit, the body path through `roll_if_full`, which rolls or refuses
    // `refusals_full`, and a pointer brings no rows. The writer still refuses
    // by name, in every build, rather than trust that: a `debug_assert` here
    // would let a release build write records past the room (repair round 1).
    // There is no branch below that could skip the rows.
    if !rows_fit(state.refusals_end, refused.len()) {
        return Err(KeepError::RefusalsFull {
            pack: at.refusals.clone(),
            held: state.refusals_end,
            rows: refused.len() as u64,
        });
    }
    if !refused.is_empty() {
        let mut bytes = Vec::with_capacity(refused.len() * REFUSAL_LEN);
        for row in refused {
            bytes.extend_from_slice(&row.encode(ordinal));
        }
        let place = u64::from(state.refusals_end) * REFUSAL_LEN as u64;
        write_synced(&at.refusals, place, &bytes)?;
        receipt.refusals_from = state.refusals_end;
        receipt.refusals = u32::try_from(refused.len()).unwrap_or(u32::MAX);
    }
    // A receipt at ordinal 0 is its file's first: `write_synced` syncs the
    // directory then, as it does for every pack file's first bytes.
    write_synced(
        &at.receipts,
        u64::from(ordinal) * RECEIPT_LEN as u64,
        &receipt.encode(),
    )?;
    Ok(Placed {
        month,
        segment,
        ordinal,
        receipt,
        rolled,
    })
}

fn place_receipt_only(
    month: YearMonth,
    (segment, at, state, rolled): (u8, Names, Segment, bool),
    pointer: Receipt,
) -> Result<Placed, KeepError> {
    let ordinal = state.count();
    // A POINTER OWNS NO RECORDS OF ITS OWN: its `refusals_from` and
    // `refusals` name the first month's, beside its body. ITS `segment` AND
    // `body_of` NAME THE FIRST MONTH'S RECEIPT, a `kept=0` pointer's too: one
    // that took its own segment lost which of the first month's segments
    // held that receipt once the month had rolled (repair round 2).
    write_refusals_then_receipt(&at, &state, ordinal, pointer, &[], segment, month, rolled)
}

#[cfg(test)]
#[path = "response_keep_tests.rs"]
mod tests;
