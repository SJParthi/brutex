//! Every run, on disk, keyed by the identity that names it.
//!
//! # What this closes
//!
//! A sweep printed its answer to stdout and the answer was gone. Nothing in the
//! workspace could say what had been run, when, on which span, or what it found
//! — so "which threshold did best on NIFTY 15-minute" was a question that could
//! only be answered by running everything again. `CLAUDE.md` §3 rule 3 requires
//! every computation to record its identity; it was computed and printed, and
//! printed is not recorded.
//!
//! # The shape, and why it is not a database
//!
//! One append-only file of FIXED-STRIDE records. §4 bans a query planner and a
//! dynamic schema, and the reason is the same for both: *the path is the index*,
//! and *a new field is a new file version at its own stride*. This follows
//! `store::format`'s layout exactly — a magic, a version, then records at a
//! constant stride, so the address of record *i* is `HEADER + i · STRIDE`: an
//! add and a multiply.
//!
//! | Operation | Cost | How |
//! |---|---|---|
//! | append | **O(1)** | seek to end, one write of one stride |
//! | read record *i* | **O(1)** | seek to `HEADER + i·STRIDE`, one read |
//! | count | **O(1)** | `(file_len - HEADER) / STRIDE`, no walk |
//! | duplicate check | **O(1)** amortised | a `HashSet` of identities, built once at open |
//!
//! The duplicate check is the only one that is not O(1) *outright*, and the cost
//! is stated rather than hidden: building the set is one pass over the file at
//! open. That pass is O(runs), runs is a few thousand after years of operating,
//! and it happens once per process — never per bar, never per candidate, so it
//! is not on the path §3 rule 4 governs.
//!
//! # Append-only, because §3 rule 8 says so
//!
//! No record is ever rewritten and no field is reused. A rerun of an identity
//! that is already present is REFUSED rather than overwritten: §3 rule 5 says
//! same inputs give same outputs, so a second run of one identity has nothing
//! new to say, and letting it overwrite would destroy the first run's timestamp
//! for no gain.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

/// Why a result could not be written or read, in the operator's words.
pub type Refusal = String;

/// `BRUTEXRS`, so a file that is not this one is refused before it is parsed.
pub(crate) const MAGIC: [u8; 8] = *b"BRUTEXRS";

/// Version TWO: version one had no seal. A new field is a new version at its own
/// stride, never a
/// widened record — `CLAUDE.md` §4 and §3 rule 8 together.
/// The version before this one, which this build READS and never writes.
///
/// Named rather than written as `2` at its two use sites, so a future version
/// 4 that also wants to read 3 changes one constant and not a scattering of
/// literals whose meaning is only clear in context.
const VERSION_V2: u32 = 2;

const VERSION: u32 = 3;

/// The stride a given format version addresses records at.
///
/// One function and not a scattering of `if version ==` so that a third
/// version adds one arm here rather than one branch per call site. An unknown
/// version cannot reach this -- `open` refuses it -- so the fallback arm is
/// the older format and never a guess.
const fn stride_of(version: u32) -> u64 {
    if version == VERSION {
        STRIDE
    } else {
        STRIDE_V2
    }
}

/// Magic, version, and four bytes reserved so the header is a round sixteen.
const HEADER: u64 = 16;

/// [`HEADER`] as a `usize`, for the header array. Same reason as
/// [`STRIDE_BYTES`]: a cast would be a narrowing on a 32-bit target.
pub(crate) const HEADER_BYTES: usize = 16;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// Bytes per record. Every field below is fixed-width; nothing here is a string
/// of unknown length, because a variable record has no stride and therefore no
/// O(1) address.
///
/// # 213, and the last eight are the seal
///
/// Version 1 was 205 and carried no integrity check, so a record damaged after
/// it was written parsed cleanly and rendered as a run that never happened.
/// The eight added bytes are `blake3` over the other 205 -- see [`SEAL_BYTES`].
///
/// # It was 176 for about ten minutes
///
/// This constant was declared before the fields were counted, and
/// `the_stride_is_exactly_what_the_writer_writes` failed on its first run with
/// `left: 176, right: 205`. That is the entire reason the test exists: had the
/// writer emitted 205 bytes while every address was computed from 176, record 1
/// would have been read starting 29 bytes into record 0 -- and it would still
/// PARSE, because every byte pattern is a legal value of its type. The file
/// would have been silently and completely wrong, with no error anywhere.
///
/// Not padded to a round number: §4 says a new field is a new file version at
/// its own stride, so reserved space would be space for a change the format
/// does not permit.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub const STRIDE: u64 = 261;

/// [`STRIDE`] as a `usize`, for the record arrays.
///
/// Declared rather than cast: `STRIDE as usize` is a narrowing on a 32-bit
/// target and clippy is right to refuse it. Two constants that must agree, and
/// a `const` assertion that they do -- which a cast could not give.
pub const STRIDE_BYTES: usize = 261;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// Bytes of a record the seal covers: everything before the seal itself.
///
/// The seal cannot cover itself, so this is [`STRIDE_BYTES`] less [`SEAL_BYTES`]
/// and it is also the stride version 1 used — the eight new bytes are the whole
/// of the difference between the two versions.
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

/// Bytes of `blake3` kept as the seal.
///
/// Eight and not thirty-two: the seal answers *were these bytes written
/// together*, and a 64-bit digest gives a chance of about 1 in 1.8e19 that a
/// corrupted record passes. A ledger holds thousands of records, not
/// quintillions. Thirty-two bytes would spend 24 more per record to move a
/// number that is already far below every other risk in the system.
const SEAL_BYTES: usize = 8;

/// The stride of the version before this one.
///
/// # Why an old format stays readable
///
/// `CLAUDE.md` §3 rule 8 is append-only history: *"store format versions are
/// never mutated in place"*. Refusing to READ one is a different thing from
/// refusing to mutate it, and the first build of version 3 did both -- a
/// fourteen-hour sweep still writing version 2 would have produced a file the
/// new binary would not open, and every figure in it would have been lost to a
/// version check rather than to anything wrong with the data.
///
/// So version 2 is READ and never APPENDED TO. A version-2 record is a whole
/// record; it simply predates the mask, and the mask reads as six zero words,
/// which the report already renders as "no combination was recorded".
const STRIDE_V2: u64 = 213;

/// [`STRIDE_V2`] as a `usize`, for the record arrays.
const STRIDE_BYTES_V2: usize = 213;

const _: () = assert!(STRIDE_BYTES_V2 as u64 == STRIDE_V2);

/// Version 2's payload, which is its stride less the same eight-byte seal.
const PAYLOAD_BYTES_V2: usize = STRIDE_BYTES_V2 - SEAL_BYTES;

/// Version 2's payload is EXACTLY version 3's, up to the mask.
///
/// This is the assumption the whole read path rests on, and it is checked at
/// compile time rather than trusted: the mask was appended as the LAST field
/// before the seal, so every version-2 offset is unchanged in version 3. Were a
/// field ever inserted rather than appended, this assertion fails the build
/// instead of the reader silently decoding one field into another.
const _: () = assert!(PAYLOAD_BYTES_V2 + 8 * 6 == PAYLOAD_BYTES);

/// A version-2 record widened into version 3's byte layout.
///
/// The payload is copied verbatim and the mask words are left zero. The seal is
/// NOT copied: it covers 205 bytes and version 3's covers 253, so carrying it
/// across would make every widened record fail its own seal check. The seal is
/// verified against the ORIGINAL bytes, before this runs.
fn widen_v2(raw: &[u8; STRIDE_BYTES_V2]) -> [u8; STRIDE_BYTES] {
    let mut wide = [0_u8; STRIDE_BYTES];
    // Both slices are compile-time-known lengths and `PAYLOAD_BYTES_V2` is less
    // than `STRIDE_BYTES`, proven by the assertion above.
    if let (Some(target), Some(source)) = (
        wide.get_mut(..PAYLOAD_BYTES_V2),
        raw.get(..PAYLOAD_BYTES_V2),
    ) {
        target.copy_from_slice(source);
    }
    wide
}

/// Whether a version-2 record matches the seal written with it.
///
/// Separate from [`Record::seal_matches`] because the two hash different
/// lengths: version 2's seal covers 205 bytes and version 3's covers 253. One
/// function taking a length would be the dynamic schema §4 bans; two constants
/// and two functions is the whole of it.
fn seal_matches_v2(raw: &[u8; STRIDE_BYTES_V2]) -> bool {
    let mut hasher = brutex_core::blake3::Hasher::new();
    let Some(payload) = raw.get(..PAYLOAD_BYTES_V2) else {
        return false;
    };
    hasher.update(payload);
    let full = hasher.finalize();
    full.get(..SEAL_BYTES) == raw.get(PAYLOAD_BYTES_V2..STRIDE_BYTES_V2)
}

/// One record at a byte offset, widened to the current layout, and whether its
/// seal held.
///
/// # Why one function for both versions
///
/// Two call sites need this -- `open`'s identity pass and `read_locked` -- and
/// each would otherwise carry its own `if version` with its own array size and
/// its own seal call. Two copies of a branch that decides how to interpret
/// bytes on disk is how a reader comes to decode one version with the other's
/// offsets, which parses cleanly and renders as a run that never happened.
///
/// The seal is returned rather than refused here, because the two callers want
/// different things from a bad seal: `read_locked` refuses that record by name,
/// and `open`'s identity pass skips it -- a damaged record still occupies its
/// stride and the records after it are still addressable.
fn read_at(file: &mut File, at: u64, version: u32) -> Result<([u8; STRIDE_BYTES], bool), Refusal> {
    if version == VERSION {
        let mut raw = [0_u8; STRIDE_BYTES];
        file.seek(SeekFrom::Start(at))
            .and_then(|_| file.read_exact(&mut raw))
            .map_err(|why| format!("record at byte {at} could not be read: {why}"))?;
        let sealed = Record::seal_matches(&raw);
        return Ok((raw, sealed));
    }
    let mut raw = [0_u8; STRIDE_BYTES_V2];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("record at byte {at} could not be read: {why}"))?;
    // SEALED AGAINST ITS OWN LENGTH, then widened. The other order would hash
    // 253 bytes of which 48 are zeroes this function invented, and every
    // version-2 record in existence would fail.
    let sealed = seal_matches_v2(&raw);
    Ok((widen_v2(&raw), sealed))
}

/// Eight bytes of `blake3` over the record's payload.
///
/// Reads `raw[..PAYLOAD_BYTES]` and ignores whatever occupies the seal slot, so
/// the same function serves the writer computing a seal and the reader checking
/// one.
fn seal_of(raw: &[u8; STRIDE_BYTES]) -> [u8; SEAL_BYTES] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(&raw[..PAYLOAD_BYTES]);
    let full = hasher.finalize();
    let mut out = [0_u8; SEAL_BYTES];
    out.copy_from_slice(&full[..SEAL_BYTES]);
    out
}

/// One completed run, as it is stored.
///
/// Sixteen-byte text fields hold the instrument and the rung. They are FIXED and
/// zero-padded rather than length-prefixed: a length prefix makes the record
/// variable, and a variable record cannot be addressed by multiplication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    /// `blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ digest ‖
    /// vocab ‖ commit ‖ feed)` — the nine terms §3 rule 3 names.
    pub identity: [u8; 32],
    /// When the run finished, microseconds since the epoch.
    pub finished_micros: i64,
    /// The feed's directory word, zero-padded.
    pub feed: [u8; 16],
    /// The instrument, zero-padded.
    pub underlying: [u8; 16],
    /// The SIGNAL rung, zero-padded. Execution is always one-minute.
    pub timeframe: [u8; 16],
    /// First month of the span.
    pub from_year: u16,
    /// First month of the span.
    pub from_month: u8,
    /// Last month of the span.
    pub to_year: u16,
    /// Last month of the span.
    pub to_month: u8,
    /// Months the span asked for.
    pub months_asked: u32,
    /// Months the store actually held. Below `months_asked` means a HOLE, and
    /// every figure in this record is over a shorter sample.
    pub months_found: u32,
    /// Signal bars swept.
    pub bars: u64,
    /// The support threshold applied.
    pub min_hits: u64,
    /// Combinations the ladder produced.
    pub combinations: u64,
    /// Deepest level reached.
    pub depth: u32,
    /// `1` when a budget stopped the walk short, so `depth` is PARTIAL and
    /// `combinations` covers less of the ladder than it appears to.
    pub halted: u8,
    /// Round trips the chosen combination took.
    pub trades: u64,
    /// Its total under worst-case fills, in paisa. **The figure selection ranks
    /// on**, so the one to compare runs by.
    pub pessimistic: i64,
    /// Its total under best-case fills, in paisa.
    pub optimistic: i64,
    /// Worst single round trip, in paisa.
    pub worst_trade: i64,
    /// Worst peak-to-trough, in paisa.
    pub max_drawdown: i64,
    /// Winners' adverse excursion, ppm — the tightest stop that would not have
    /// killed a winner.
    pub winner_mae: i64,
    /// Winners' favourable excursion, ppm.
    pub winner_mfe: i64,
    /// Every trade's adverse excursion, ppm.
    pub all_mae: i64,
    /// Chosen exit rungs, `-1` for "no rung": stop, target, TSL, TTP arm, TTP
    /// trail. Five axes, so five fields — a packed integer would be a schema
    /// inside a schema.
    pub exit_rungs: [i16; 5],
    /// The combination itself, as its six mask words.
    ///
    /// # What version 2 could not say
    ///
    /// A record carried `identity` -- a blake3 over the nine terms §3 rule 3
    /// names -- and that hash identifies the RUN. It cannot be turned back into
    /// the conditions, so `cli results` could print the money a combination
    /// made and never which conditions made it. The names existed only in the
    /// audit's own output, which is stdout and gone.
    ///
    /// Forty-eight bytes, and `vocab` turns them back into names on read. The
    /// ledger becomes the thing it was always supposed to be: a permanent
    /// record of what was found, not just of what it was worth.
    ///
    /// Six `u64` and not a variable list of positions, for the reason the
    /// header gives: a variable record has no stride and therefore no O(1)
    /// address.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub mask_words: [u64; 6],
}

impl Record {
    /// The record as its exact `STRIDE` bytes, little-endian throughout.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "every index below is a constant offset into a fixed-size \
                  array whose length is asserted against STRIDE by \
                  `the_stride_is_exactly_what_the_writer_writes`"
    )]
    pub fn to_bytes(&self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&self.identity, &mut at);
        put(&self.finished_micros.to_le_bytes(), &mut at);
        put(&self.feed, &mut at);
        put(&self.underlying, &mut at);
        put(&self.timeframe, &mut at);
        put(&self.from_year.to_le_bytes(), &mut at);
        put(&[self.from_month], &mut at);
        put(&self.to_year.to_le_bytes(), &mut at);
        put(&[self.to_month], &mut at);
        put(&self.months_asked.to_le_bytes(), &mut at);
        put(&self.months_found.to_le_bytes(), &mut at);
        put(&self.bars.to_le_bytes(), &mut at);
        put(&self.min_hits.to_le_bytes(), &mut at);
        put(&self.combinations.to_le_bytes(), &mut at);
        put(&self.depth.to_le_bytes(), &mut at);
        put(&[self.halted], &mut at);
        put(&self.trades.to_le_bytes(), &mut at);
        put(&self.pessimistic.to_le_bytes(), &mut at);
        put(&self.optimistic.to_le_bytes(), &mut at);
        put(&self.worst_trade.to_le_bytes(), &mut at);
        put(&self.max_drawdown.to_le_bytes(), &mut at);
        put(&self.winner_mae.to_le_bytes(), &mut at);
        put(&self.winner_mfe.to_le_bytes(), &mut at);
        put(&self.all_mae.to_le_bytes(), &mut at);
        for rung in self.exit_rungs {
            put(&rung.to_le_bytes(), &mut at);
        }
        // LAST, so every version-2 offset is unchanged. A field inserted in the
        // middle would move every one after it, and a version-2 file read at
        // version 3 would parse cleanly into nonsense -- which the version check
        // prevents, but only because it refuses the file outright.
        for word in self.mask_words {
            put(&word.to_le_bytes(), &mut at);
        }
        debug_assert_eq!(at, PAYLOAD_BYTES, "every field is written before the seal");
        let seal = seal_of(&out);
        out[PAYLOAD_BYTES..STRIDE_BYTES].copy_from_slice(&seal);
        out
    }

    /// Does this record's seal match the bytes it covers?
    ///
    /// # What a seal catches that a stride check cannot
    ///
    /// `Results::open` refuses a ledger whose payload is not a whole number of
    /// strides, which catches an interrupted write that stopped mid-record. It
    /// cannot catch one that stopped exactly ON a boundary, nor a bit flipped
    /// on disk years later, nor a byte written by something that is not this
    /// program — every one of those still measures as a whole record, and every
    /// byte pattern is a legal value of its type, so the parse succeeds and the
    /// nonsense is rendered as data.
    ///
    /// The seal is what closes that: eight bytes of `blake3` over the 205 the
    /// record actually carries. It is not a security claim — nothing here is
    /// defending against a forger — it is a claim that these bytes are the ones
    /// that were written together.
    ///
    /// # Cost
    ///
    /// **O(1).** One hash of a FIXED 205 bytes, per record read or written.
    /// Not per bar and not per candidate, so it is not on the path §3 rule 4
    /// governs.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn seal_matches(raw: &[u8; STRIDE_BYTES]) -> bool {
        seal_of(raw) == raw[PAYLOAD_BYTES..STRIDE_BYTES]
    }

    /// The record back from its exact `STRIDE` bytes.
    ///
    /// Infallible by construction: every field is fixed-width and every byte
    /// pattern is a legal value of its type. A record whose CONTENT is
    /// nonsensical is a separate question and is not answered by pretending the
    /// parse can fail.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "the same constant offsets `to_bytes` writes, over an array of \
                  the same fixed length"
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Self {
        let mut at = 0_usize;
        let take = |n: usize, at: &mut usize| {
            let slice = &raw[*at..*at + n];
            *at += n;
            slice
        };
        let mut id = [0_u8; 32];
        id.copy_from_slice(take(32, &mut at));
        let finished_micros = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mut feed = [0_u8; 16];
        feed.copy_from_slice(take(16, &mut at));
        let mut underlying = [0_u8; 16];
        underlying.copy_from_slice(take(16, &mut at));
        let mut timeframe = [0_u8; 16];
        timeframe.copy_from_slice(take(16, &mut at));
        let from_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let from_month = take(1, &mut at).first().copied().unwrap_or(0);
        let to_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let to_month = take(1, &mut at).first().copied().unwrap_or(0);
        let months_asked = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let months_found = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let bars = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let min_hits = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let combinations = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let depth = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let halted = take(1, &mut at).first().copied().unwrap_or(0);
        let trades = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let pessimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let optimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let worst_trade = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let max_drawdown = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let winner_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mfe_winners = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let all_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mut exit_rungs = [0_i16; 5];
        for slot in &mut exit_rungs {
            *slot = i16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        }
        let mut mask_words = [0_u64; 6];
        for slot in &mut mask_words {
            *slot = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        }
        Self {
            identity: id,
            finished_micros,
            feed,
            underlying,
            timeframe,
            from_year,
            from_month,
            to_year,
            to_month,
            months_asked,
            months_found,
            bars,
            min_hits,
            combinations,
            depth,
            halted,
            trades,
            pessimistic,
            optimistic,
            worst_trade,
            max_drawdown,
            winner_mae,
            winner_mfe: mfe_winners,
            all_mae,
            exit_rungs,
            mask_words,
        }
    }

    /// The identity as lowercase hex, which is how a run is named everywhere
    /// else in this workspace.
    #[must_use]
    pub fn identity_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for byte in self.identity {
            let _ = write!(s, "{byte:02x}");
        }
        s
    }
}

use core::fmt::Write as _;

/// A fixed-width text field, truncated rather than refused.
///
/// Truncation is safe here and nowhere else: these fields are for a HUMAN
/// reading a listing, and the identity is what actually names the run. A refusal
/// would throw away a completed sweep because an instrument name was long.
#[must_use]
pub fn field(text: &str) -> [u8; 16] {
    let mut out = [0_u8; 16];
    for (slot, byte) in out.iter_mut().zip(text.bytes()) {
        *slot = byte;
    }
    out
}

/// A fixed-width text field, back to a string with its padding removed.
#[must_use]
pub fn read_field(raw: &[u8; 16]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(raw.get(..end).unwrap_or(&[])).into_owned()
}

/// The append-only file of every run this store has recorded.
///
/// `Debug` shows the handle and the identity count, not the identities: a
/// listing of thousands of digests in a panic message helps nobody.
#[derive(Debug)]
pub struct Results {
    file: File,
    /// The version the FILE carries, which may be older than this build writes.
    ///
    /// Version 2 is readable and NOT appendable -- see [`STRIDE_V2`]. Held on
    /// the handle rather than re-read per call: the header is read once at
    /// open, and a version that could change under a live handle would mean
    /// another process rewrote the header, which is not a thing this format
    /// permits.
    version: u32,
    seen: std::collections::HashSet<[u8; 32]>,
    /// Byte offset this process has absorbed identities up to.
    ///
    /// `seen` is built once at open, so it knows nothing about a record ANOTHER
    /// process appended afterwards. `append` re-scans from here under the file
    /// lock before it decides a run is new -- see [`Results::append`].
    scanned: u64,
}

/// Write the sixteen-byte header of a fresh ledger, and prove it was kept.
///
/// Split out of [`Results::open`] to keep that function inside its line budget
/// once the read-back arrived — a budget met by deleting the read-back instead
/// would have traded a real check for a lint.
fn write_fresh_header(file: &mut File, path: &Path) -> Result<(), Refusal> {
    let mut header = [0_u8; HEADER_BYTES];
    header
        .get_mut(..8)
        .ok_or_else(|| "the header is shorter than its magic".to_owned())?
        .copy_from_slice(&MAGIC);
    header
        .get_mut(8..12)
        .ok_or_else(|| "the header is shorter than its version".to_owned())?
        .copy_from_slice(&VERSION.to_le_bytes());
    file.write_all(&header)
        .map_err(|why| format!("the header could not be written: {why}"))?;
    // THE HEADER IS READ BACK, BECAUSE A SUCCESSFUL WRITE IS NOT PROOF
    // THAT ANYTHING WAS STORED.
    //
    // # The failure this closes, reproduced on a scratch store
    //
    // Point `runs.bin` at `/dev/null` — a symlink, a stale path, a
    // device node — and every call here SUCCEEDS. The header is written
    // and discarded, the file measures zero, and the next open writes a
    // header again. `cli results` reported `rows 0` with no refusal and
    // exit 0, and a sweep would have recorded hours of work into the
    // void and called it done.
    //
    // The same silence covers anything that accepts a write without
    // keeping it: a full filesystem that reports success until fsync, a
    // path on a volume that unmounted, a file another process truncated
    // between the create and the write.
    //
    // One seek and one read of sixteen bytes, once per FRESH ledger.
    // Not per record and not per bar.
    file.flush()
        .map_err(|why| format!("the header could not be flushed: {why}"))?;
    let mut written_back = [0_u8; HEADER_BYTES];
    let stored = file
        .seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut written_back))
        .is_ok();
    if !stored || written_back != header {
        return Err(format!(
            "{} accepted a header and did not keep it: sixteen bytes \
             were written and reading them back did not return them. \
             The path is not a file that stores what it is given — a \
             link to a device such as /dev/null, a volume that has gone \
             away, or a filesystem with no room. Nothing is recorded \
             here, and a run that believed otherwise would lose every \
             result it produced.",
            path.display()
        ));
    }
    Ok(())
}

impl Results {
    /// Opens, or creates, the results file beneath `root`.
    ///
    /// # Errors
    ///
    /// An unwritable directory, a file whose magic is not this format's, or a
    /// version this build does not know. Each refuses rather than being
    /// repaired: a file that is not this one must not be appended to.
    pub fn open(root: &Path) -> Result<Self, Refusal> {
        let dir = root.join("results");
        std::fs::create_dir_all(&dir)
            .map_err(|why| format!("the results directory could not be made: {why}"))?;
        let path = Self::path(root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;

        let len = file
            .metadata()
            .map_err(|why| format!("the results file could not be measured: {why}"))?
            .len();
        // The branch YIELDS the version rather than setting a mutable above it:
        // a fresh file is the version this build writes, and an existing one is
        // whatever its header says. Every read below is scoped to that answer.
        let version = if len == 0 {
            write_fresh_header(&mut file, &path)?;
            VERSION
        } else {
            let mut header = [0_u8; HEADER_BYTES];
            file.seek(SeekFrom::Start(0))
                .and_then(|_| file.read_exact(&mut header))
                .map_err(|why| format!("the header could not be read: {why}"))?;
            if header.get(..8) != Some(&MAGIC) {
                return Err(format!(
                    "{} is not a brutex results file: its first eight bytes are \
                     not `BRUTEXRS`. Nothing was written.",
                    path.display()
                ));
            }
            let version = u32::from_le_bytes(
                header
                    .get(8..12)
                    .and_then(|s| s.try_into().ok())
                    .unwrap_or([0; 4]),
            );
            // READ, BUT NOT APPENDED TO. See `STRIDE_V2`.
            //
            // §3 rule 8 protects a stored format's history; refusing to OPEN
            // one is a different act from refusing to mutate it, and the first
            // build of version 3 did both. A sweep that had been running for
            // fourteen hours was still writing version 2, and every figure in
            // it would have been lost to a version check rather than to
            // anything wrong with the data.
            //
            // A version this build has never written is still refused: the
            // offsets of a FUTURE format are unknown, and guessing them is how
            // a reader renders a run that never happened.
            if version != VERSION && version != VERSION_V2 {
                return Err(format!(
                    "{} is version {version}. This build reads {VERSION_V2} and \
                     {VERSION} and writes {VERSION}; it does not know this \
                     one's field offsets and will not guess them.",
                    path.display()
                ));
            }
            // A TRAILING PART-RECORD IS A REFUSAL, NOT A ROUNDING.
            //
            // # The failure this closes, reproduced before it was written
            //
            // `len()` is `(file_len - HEADER) / STRIDE`, and integer division
            // DISCARDS the remainder. A machine that lost power partway through
            // an append left a part-record that no reader ever mentioned:
            // measured on a copy of a real ledger, appending 100 bytes and
            // listing it printed the same fourteen rows, said nothing, and
            // exited 0.
            //
            // Silence was only half of it. `append` seeks `SeekFrom::End(0)`,
            // so the NEXT record lands after the orphan bytes and every record
            // from then on is offset by however many they were — while `read`
            // still seeks `HEADER + index * STRIDE`. The same measurement showed
            // what that renders as: a row with `18,446,744,073,709,551,615`
            // combinations, no vendor, no rung, months `4294901760/0`, and a
            // total of -₹0.01. It was counted in the row total and it was
            // printed as data.
            //
            // Whether such a row can also WIN `BEST COMPLETE RUN` is decided by
            // whichever byte lands on `halted`. In that run it happened to be
            // non-zero and the row was skipped — luck, not design.
            //
            // Refused rather than healed. Truncating the orphan would be a
            // silent repair of a file whose history §3 rule 8 protects, and §4
            // bans a fallback that hides a failure. The count of intact records
            // is named so an operator can see exactly what survived.
            let payload = len.saturating_sub(HEADER);
            let stride = stride_of(version);
            let orphan = payload % stride;
            if orphan != 0 {
                return Err(format!(
                    "{} ends with {orphan} bytes that are not a whole record: \
                     {} complete records occupy {} bytes after the {HEADER}-byte \
                     header, and the file is {len}. A write was interrupted. \
                     Nothing here is repaired automatically — the intact records \
                     are readable and the orphan bytes are not, and truncating \
                     them is a decision about history that belongs to you.",
                    path.display(),
                    payload / stride,
                    payload - orphan,
                ));
            }
            version
        };
        let stride = stride_of(version);

        // ONE PASS, ONCE, AT OPEN. Stated rather than hidden: this is O(runs)
        // and every other operation on this type is O(1). It is not on the
        // per-bar or per-candidate path §3 rule 4 governs.
        //
        // AND IT IS PRE-SIZED, because the count is known before the loop runs.
        // `docs/07-o1-architecture.md` law 2: growth is the ONLY source of O(n)
        // in a hash table, so a set that rehashes while walking makes this
        // open's cost depend on where `with_capacity` happened to round rather
        // than on how many rows there are. Both arms of `stride_of` return a
        // non-zero `const`, so the divide is defined; `saturating_sub` covers a
        // file shorter than its own header, which the loop below already
        // tolerates by never entering.
        let records = len.saturating_sub(HEADER) / stride;
        let mut seen =
            std::collections::HashSet::with_capacity(usize::try_from(records).unwrap_or(0));
        let mut at = HEADER;
        while at + stride <= len {
            // A DAMAGED RECORD IS SKIPPED HERE, NOT REFUSED.
            //
            // This pass exists to learn which identities are already present so
            // `append` can refuse a rerun. A record whose seal failed cannot be
            // trusted to say what its identity IS, so admitting it would let a
            // corrupted 32 bytes block a legitimate run from ever being
            // recorded. It still occupies its stride, so the records after it
            // stay addressable, and `read` refuses it BY NAME when asked for.
            let (raw, sealed) = read_at(&mut file, at, version)?;
            if sealed {
                seen.insert(Record::from_bytes(&raw).identity);
            }
            at = at.saturating_add(stride);
        }
        Ok(Self {
            file,
            seen,
            scanned: at,
            version,
        })
    }

    /// Where the file lives: `<store>/results/runs.bin`.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("runs.bin")
    }

    /// The stride of the file THIS handle opened, which is not always the
    /// stride this build writes.
    const fn stride(&self) -> u64 {
        stride_of(self.version)
    }

    /// How many runs are recorded. **O(1)** — a division, not a walk.
    ///
    /// # Errors
    ///
    /// An unreadable file handle.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn len(&self) -> Result<u64, Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("the results file could not be measured: {why}"))?
            .len();
        // THE FILE'S OWN STRIDE, not this build's. A version-2 ledger addresses
        // records every 213 bytes, and dividing its length by 261 reports fewer
        // runs than it holds -- silently, because the arithmetic succeeds.
        Ok(len.saturating_sub(HEADER) / self.stride())
    }

    /// Whether nothing has been recorded yet.
    ///
    /// # Errors
    ///
    /// Every error [`Self::len`] returns.
    pub fn is_empty(&self) -> Result<bool, Refusal> {
        Ok(self.len()? == 0)
    }

    /// Whether this identity is already recorded. **O(1).**
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn holds(&self, identity: &[u8; 32]) -> bool {
        self.seen.contains(identity)
    }

    /// Appends one run. **O(1)** — a seek to the end and one write.
    ///
    /// # Errors
    ///
    /// A duplicate identity, or an unwritable file. A duplicate REFUSES rather
    /// than overwriting: §3 rule 5 makes a rerun byte-identical, so the second
    /// run has nothing to add, and overwriting would destroy the first one's
    /// timestamp for no gain.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn append(&mut self, record: &Record) -> Result<u64, Refusal> {
        // AN EXCLUSIVE FILE LOCK, BECAUSE A PROCESS MUTEX GUARDS THE WRONG
        // BOUNDARY.
        //
        // `cli`'s `LEDGER` mutex serialises the threads of ONE process, and its
        // own comment says so: *"Two PROCESSES appending at once is a different
        // problem and is not solved by this ... a second `range-all` should not
        // be started while one is running."* That is an honest statement of a
        // gap, but the thing enforcing it was a sentence in a source file, in a
        // tree where two sessions and an HTTP server all reach the same store.
        //
        // # What two writers cost without this
        //
        // `append` seeks to the end and then writes. Between those two calls
        // another process can do the same, so both compute the same offset and
        // the second write lands ON TOP of the first: a whole, correctly sealed
        // record that silently replaces another whole, correctly sealed record.
        // The seal cannot see it -- both records are internally consistent --
        // and the count still reads as one record, so the loss leaves no trace
        // anywhere. That is a run vanishing, which is the one thing an
        // append-only ledger exists to prevent.
        //
        // # And the duplicate check needs it too
        //
        // `seen` is built once at open, so it cannot know about a record another
        // process appended since. Locking without re-scanning would fix the
        // overwrite and leave two copies of one identity, which §3 rule 5 and
        // `self.holds` both refuse in-process.
        // AN OLDER FORMAT IS READ AND NEVER WRITTEN TO, and the refusal comes
        // before the lock so a doomed append never blocks a reader.
        //
        // §3 rule 8: *"store format versions are never mutated in place"*.
        // Appending a 261-byte record to a file addressed every 213 bytes would
        // do exactly that, and every record after it would decode with the
        // wrong offsets -- which parses cleanly, because every byte pattern
        // here is a legal record.
        if self.version != VERSION {
            return Err(format!(
                "this ledger is version {} and this build writes version \
                 {VERSION}. It is READ so nothing already recorded is lost, and \
                 it is never appended to -- a new field is a new file version \
                 at its own stride, never a widened record. Move it aside to \
                 start a version-{VERSION} ledger.",
                self.version
            ));
        }
        self.file
            .lock()
            .map_err(|why| format!("the results file could not be locked: {why}"))?;
        let out = self.append_locked(record);
        // Unlocked on both paths, including the refusals inside `append_locked`.
        // Dropping the handle would also release it, but this type outlives one
        // append by design -- a sweep opens once and records per rung.
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the results file could not be unlocked: {why}"));
        match (out, released) {
            (Ok(index), Ok(())) => Ok(index),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// [`Results::append`] with the file lock already held.
    ///
    /// Split out so the lock is released on every path this can leave by,
    /// including the refusals — an early `return` inside the locked region would
    /// otherwise strand the lock until the process exited.
    fn append_locked(&mut self, record: &Record) -> Result<u64, Refusal> {
        self.absorb_new_records()?;
        if self.holds(&record.identity) {
            return Err(format!(
                "run {} is already recorded. Same inputs give same outputs \
                 (§3 rule 5), so this run has nothing new to add and the \
                 original's timestamp is kept.",
                record.identity_hex()
            ));
        }
        let at = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("the results file could not be extended: {why}"))?;
        self.file
            .write_all(&record.to_bytes())
            .and_then(|()| self.file.flush())
            // A FAILED WRITE IS ROLLED BACK, AND THIS IS THE ONE PLACE THAT IS
            // NOT A SILENT REPAIR.
            //
            // `write_all` on a full filesystem can put SOME of the 213 bytes
            // down before it fails, and `Results::open` refuses a ledger whose
            // tail is a part-record — correctly, since it cannot know what put
            // the bytes there. So one `ENOSPC` would leave a ledger that every
            // later process refuses to open, over bytes that were never a record
            // and that nobody wants.
            //
            // Here the cause IS known: this call just failed, and `at` is where
            // the file ended before it started. Truncating back to `at` removes
            // bytes this function wrote and nothing else. That is not the
            // history §3 rule 8 protects — a record that was never completed was
            // never a record — and it is the difference between a disk that
            // filled up and a ledger that has to be repaired by hand.
            //
            // The refusal still names the write failure, so nothing is hidden.
            // If the rollback ITSELF fails, both reasons are reported: an
            // operator facing a truncate that cannot run needs to know the tail
            // is still there.
            .map_err(|why| match self.file.set_len(at) {
                Ok(()) => format!(
                    "the record could not be written: {why}. The partial write \
                     was rolled back, so the ledger still ends on a whole record."
                ),
                Err(and) => format!(
                    "the record could not be written: {why}. Rolling the partial \
                     write back ALSO failed: {and}. The file may now end mid-record \
                     and will be refused on the next open until its tail is cut \
                     back to byte {at}."
                ),
            })?;
        self.seen.insert(record.identity);
        self.scanned = at.saturating_add(STRIDE);
        Ok(at.saturating_sub(HEADER) / STRIDE)
    }

    /// Absorb identities written by anyone else since this handle last looked.
    ///
    /// # Cost
    ///
    /// **O(records appended by others)**, which is zero on the overwhelmingly
    /// common path of one writer — the `while` does not execute and this is a
    /// length check. It is never per bar and never per candidate.
    ///
    /// A part-record at the tail is left alone rather than refused here:
    /// `Results::open` already refuses one, and a writer that finds a torn tail
    /// mid-run should say so through the same message rather than a second one
    /// worded differently.
    fn absorb_new_records(&mut self) -> Result<(), Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("the results file could not be measured: {why}"))?
            .len();
        while self.scanned.saturating_add(STRIDE) <= len {
            let mut raw = [0_u8; STRIDE_BYTES];
            let at = self.scanned;
            self.file
                .seek(SeekFrom::Start(at))
                .and_then(|_| self.file.read_exact(&mut raw))
                .map_err(|why| format!("record at byte {at} could not be read: {why}"))?;
            // A record another process wrote is absorbed for its IDENTITY only.
            // Its seal is not checked here: this is a duplicate test, and a
            // damaged record is refused by `read` at the moment someone tries to
            // use its numbers. Refusing an append because an unrelated row went
            // bad would stop a run for a reason that has nothing to do with it.
            self.seen.insert(Record::from_bytes(&raw).identity);
            self.scanned = at.saturating_add(STRIDE);
        }
        Ok(())
    }

    /// Reads record `index`. **O(1)** — `HEADER + index · STRIDE`, one seek.
    ///
    /// # Errors
    ///
    /// An index past the end, or an unreadable file.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn read(&mut self, index: u64) -> Result<Record, Refusal> {
        // A SHARED LOCK, BECAUSE THE SEAL WOULD OTHERWISE CRY WOLF.
        //
        // `append` holds an EXCLUSIVE lock while it writes, and this took none,
        // so a reader could land in the middle of another process's 213-byte
        // write. Before the seal existed that produced a garbled record read as
        // data; with the seal it produces a REFUSAL naming corruption — on a
        // ledger that is perfectly healthy a millisecond later.
        //
        // A false alarm is its own defect. An operator who is told a record is
        // damaged, checks, and finds it intact learns to disbelieve the message,
        // and the next one will be real. The shared lock makes a reader wait for
        // an in-progress append rather than see half of it.
        //
        // Shared and not exclusive: any number of readers may hold it at once,
        // so `cli results` and the HTTP server do not queue behind each other.
        // Only a writer excludes them.
        self.file
            .lock_shared()
            .map_err(|why| format!("the results file could not be locked for reading: {why}"))?;
        let out = self.read_locked(index);
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the results file could not be unlocked: {why}"));
        match (out, released) {
            (Ok(record), Ok(())) => Ok(record),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// [`Results::read`] with the shared lock already held.
    ///
    /// Split for the reason [`Results::append_locked`] gives: an early `return`
    /// inside the locked region would strand the lock until the process exited.
    /// It calls [`Results::len`] rather than locking again — a second `flock` on
    /// the same descriptor is not a second lock, and the `unlock` that followed
    /// it would release the one this function is standing on.
    fn read_locked(&mut self, index: u64) -> Result<Record, Refusal> {
        let count = self.len()?;
        if index >= count {
            return Err(format!(
                "record {index} does not exist: this file holds {count}."
            ));
        }
        let at = HEADER.saturating_add(index.saturating_mul(self.stride()));
        let version = self.version;
        let (raw, sealed) = read_at(&mut self.file, at, version)?;
        // THE SEAL IS CHECKED HERE, NOT AT OPEN, AND THAT IS THE POINT.
        //
        // Checking every record at open would make opening O(runs) in HASHING
        // rather than only in the identity pass, and would refuse a whole ledger
        // because one record in the middle went bad. Checking at read refuses
        // exactly the record that is unreadable and leaves every other one
        // available — which is what an operator needs when a disk has damaged
        // one row out of thousands.
        //
        // `from_bytes` is infallible by construction: every byte pattern is a
        // legal value of its type, so a damaged record PARSES and renders as
        // data. The seal is the only thing standing between that and a number
        // an operator would act on.
        if !sealed {
            return Err(format!(
                "record {index} does not match its seal: the eight bytes written \
                 with it do not describe the {PAYLOAD_BYTES} bytes now on disk. \
                 The record was damaged after it was written -- an interrupted \
                 write that stopped on a stride boundary, a bad sector, or \
                 something that is not this program writing to this file. It is \
                 NOT repaired and NOT skipped silently, because every byte \
                 pattern here parses into a legal record and would render as a \
                 run that never happened."
            ));
        }
        Ok(Record::from_bytes(&raw))
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail. `indexing_slicing` is NOT listed: \
              nothing here indexes, and an unfulfilled expectation is itself a \
              warning"
)]
mod tests {
    use super::{
        HEADER, HEADER_BYTES, MAGIC, PAYLOAD_BYTES, PAYLOAD_BYTES_V2, Record, Results, SEAL_BYTES,
        STRIDE, STRIDE_BYTES, STRIDE_V2, VERSION_V2, field, read_field,
    };

    fn root(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("brutex-results-{tag}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("a temp root");
        p
    }

    fn record(n: u8) -> Record {
        Record {
            identity: [n; 32],
            finished_micros: 1_785_727_500_000_000 + i64::from(n),
            feed: field("zerodha"),
            underlying: field("NIFTY"),
            timeframe: field("15min"),
            from_year: 2019,
            from_month: 12,
            to_year: 2026,
            to_month: 8,
            months_asked: 81,
            months_found: 81,
            bars: 43_875,
            min_hits: 200,
            combinations: 54_895_691,
            depth: 11,
            halted: 1,
            trades: 412,
            pessimistic: -987_654_321,
            optimistic: 123_456_789,
            worst_trade: -87_654_321,
            max_drawdown: 76_543_210,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
            // Distinct per fixture so a round trip that dropped or reordered
            // the words fails rather than passing on symmetry.
            mask_words: [u64::from(n), 0, u64::from(n) << 32, 0, 0, 7],
        }
    }

    /// THE STRIDE IS WHAT THE WRITER WRITES, and nothing else may claim it.
    ///
    /// `STRIDE` is the constant every address is computed from: the address of
    /// record *i* is `HEADER + i·STRIDE`. If the writer emitted a different
    /// number of bytes, every record after the first would be read from the
    /// wrong offset — and it would still PARSE, because every byte pattern is a
    /// legal value of its type. The file would be silently, entirely wrong.
    #[test]
    fn the_stride_is_exactly_what_the_writer_writes() {
        // Field by field, so a change to the layout has to change this sum
        // deliberately rather than by a rebuild.
        const EXPECTED: u64 = 32   // identity
            + 8                    // finished_micros
            + 16 + 16 + 16         // feed, underlying, timeframe
            + 2 + 1 + 2 + 1        // from/to year and month
            + 4 + 4                // months asked, found
            + 8 + 8 + 8            // bars, min_hits, combinations
            + 4 + 1                // depth, halted
            + 8                    // trades
            + 8 + 8                // pessimistic, optimistic
            + 8 + 8                // worst_trade, max_drawdown
            + 8 + 8 + 8            // winner_mae, winner_mfe, all_mae
            + 2 * 5                // exit_rungs
            + 8 * 6                // mask_words -- version 3's difference from version 2
            + 8; // the seal, which is version 2's whole difference from version 1
        assert_eq!(
            STRIDE, EXPECTED,
            "the declared stride must be the sum of the fields"
        );
        // The payload is the stride LESS the seal, and the seal cannot cover
        // itself. Asserted rather than assumed because `seal_of` slices by this
        // constant: were it wrong by one, every seal would be computed over a
        // window that excluded a real field, and a change to that field would
        // pass its own integrity check.
        assert_eq!(
            PAYLOAD_BYTES as u64 + SEAL_BYTES as u64,
            STRIDE,
            "the seal covers everything that is not the seal"
        );
        assert_eq!(
            record(1).to_bytes().len() as u64,
            STRIDE,
            "and the writer must emit exactly that many bytes"
        );
    }

    #[test]
    fn every_field_survives_the_round_trip() {
        // Twenty-four fields, asserted one at a time. A `assert_eq!(a, b)` on
        // the whole struct would pass if two fields were swapped in BOTH
        // directions, which is exactly the bug a hand-written codec produces.
        let want = record(7);
        let got = Record::from_bytes(&want.to_bytes());
        assert_eq!(got.identity, want.identity, "identity");
        assert_eq!(got.finished_micros, want.finished_micros, "finished_micros");
        assert_eq!(read_field(&got.feed), "zerodha", "feed");
        assert_eq!(read_field(&got.underlying), "NIFTY", "underlying");
        assert_eq!(read_field(&got.timeframe), "15min", "timeframe");
        assert_eq!(got.from_year, 2019, "from_year");
        assert_eq!(got.from_month, 12, "from_month");
        assert_eq!(got.to_year, 2026, "to_year");
        assert_eq!(got.to_month, 8, "to_month");
        assert_eq!(got.months_asked, 81, "months_asked");
        assert_eq!(got.months_found, 81, "months_found");
        assert_eq!(got.bars, 43_875, "bars");
        assert_eq!(got.min_hits, 200, "min_hits");
        assert_eq!(got.combinations, 54_895_691, "combinations");
        assert_eq!(got.depth, 11, "depth");
        assert_eq!(got.halted, 1, "halted");
        assert_eq!(got.trades, 412, "trades");
        assert_eq!(got.pessimistic, -987_654_321, "pessimistic");
        assert_eq!(got.optimistic, 123_456_789, "optimistic");
        assert_eq!(got.worst_trade, -87_654_321, "worst_trade");
        // POSITIVE, because a peak-to-trough fall is. `grid::Cell::max_drawdown`
        // is documented "Always >= 0" and asserted so, and `record_run` copies
        // it straight in — so a negative value here was a shape the engine
        // cannot produce. This fixture carrying the wrong sign is what kept
        // `range_all`'s `ret/DD` column green while it printed `-` forever: its
        // guard tested `< 0`, and only the fixtures ever satisfied it.
        assert_eq!(got.max_drawdown, 76_543_210, "max_drawdown");
        assert_eq!(got.winner_mae, 2_291, "winner_mae");
        assert_eq!(got.winner_mfe, 8_876, "winner_mfe");
        assert_eq!(got.all_mae, 2_295, "all_mae");
        assert_eq!(got.exit_rungs, [2, 3, -1, 1, 0], "exit_rungs");
        assert_eq!(got, want, "and the whole record agrees");
    }

    #[test]
    fn a_negative_exit_rung_survives_as_no_rung() {
        // `-1` means "no rung on this axis", and it is the one value that would
        // be destroyed by storing the five axes unsigned. A no-stop variant
        // reading as stop rung 65,535 would be a strategy nobody ran.
        let mut want = record(3);
        want.exit_rungs = [-1, -1, -1, -1, -1];
        assert_eq!(Record::from_bytes(&want.to_bytes()).exit_rungs, [-1; 5]);
    }

    #[test]
    fn records_are_addressed_by_multiplication_and_read_back_in_order() {
        let r = root("addr");
        let mut store = Results::open(&r).expect("a fresh store opens");
        assert!(store.is_empty().expect("measurable"));
        for n in 0..5_u8 {
            let at = store.append(&record(n)).expect("appends");
            assert_eq!(at, u64::from(n), "each append returns its own index");
        }
        assert_eq!(store.len().expect("measurable"), 5);
        for n in 0..5_u8 {
            let got = store.read(u64::from(n)).expect("reads");
            assert_eq!(got.identity, [n; 32], "record {n} is at index {n}");
        }
        // AND THE FILE IS EXACTLY THE SIZE ITS ARITHMETIC CLAIMS. A stride that
        // disagreed with the writer would show up here as a length that is not
        // a whole number of records.
        let bytes = std::fs::metadata(Results::path(&r))
            .expect("the file exists")
            .len();
        assert_eq!(bytes, HEADER + 5 * STRIDE);
    }

    #[test]
    fn a_reopened_store_appends_after_what_is_already_there() {
        // Append-only across process boundaries, which is the whole point: a run
        // recorded yesterday must still be there today, and today's must not
        // overwrite it.
        let r = root("reopen");
        {
            let mut store = Results::open(&r).expect("opens");
            store.append(&record(1)).expect("appends");
            store.append(&record(2)).expect("appends");
        }
        let mut store = Results::open(&r).expect("reopens");
        assert_eq!(store.len().expect("measurable"), 2, "nothing was lost");
        assert_eq!(
            store.append(&record(3)).expect("appends"),
            2,
            "and it appends after"
        );
        assert_eq!(
            store.read(0).expect("reads").identity,
            [1; 32],
            "the first is untouched"
        );
    }

    /// A WRITE CUT SHORT BY A DEAD MACHINE IS NAMED, NOT ROUNDED AWAY.
    ///
    /// # What this reproduces
    ///
    /// `len()` divides the payload by `STRIDE`, and integer division discards
    /// the remainder. Before this refusal existed, a ledger with a part-record
    /// on the end listed its intact rows, said nothing about the orphan, and
    /// exited 0 — so the operator's evidence that a run had been interrupted
    /// was a file size nobody looks at.
    ///
    /// The silence was the smaller half. `append` seeks to the END of the file,
    /// while `read` seeks `HEADER + index * STRIDE`, so one interrupted write
    /// puts every later record permanently out of phase with every later read.
    /// Reproduced on a copy of a real ledger, the first misaligned row rendered
    /// as `18,446,744,073,709,551,615` combinations with no vendor and no rung,
    /// and was counted in the row total as though it were a run.
    ///
    /// # Why every remainder is tried
    ///
    /// One orphan byte and `STRIDE - 1` orphan bytes are the two ends of the
    /// same defect, and a check written as `!= STRIDE` or `< STRIDE / 2` would
    /// pass one and fail the other. The loop is the cheapest way to say that the
    /// only acceptable remainder is zero.
    #[test]
    fn a_part_record_from_an_interrupted_write_is_refused_and_counted() {
        use std::io::Write as _;
        for orphan in [1_u64, 7, 100, STRIDE - 1] {
            let r = root(&format!("torn{orphan}"));
            {
                let mut store = Results::open(&r).expect("opens");
                store.append(&record(1)).expect("appends");
                store.append(&record(2)).expect("appends");
            }
            // The interrupted write itself: bytes that are not a whole record.
            let path = Results::path(&r);
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("the ledger reopens for the torn write");
            f.write_all(&vec![0_u8; usize::try_from(orphan).expect("fits")])
                .expect("the partial write lands");
            drop(f);

            let why = Results::open(&r).expect_err("a torn ledger is refused");
            assert!(
                why.contains(&orphan.to_string()),
                "the refusal must name how many bytes are orphaned, or the \
                 operator cannot tell a one-byte tear from a near-whole one: \
                 {why}"
            );
            assert!(
                why.contains('2'),
                "and how many records survived, which is what makes it \
                 actionable rather than merely alarming: {why}"
            );
        }
    }

    /// The refusal is about the REMAINDER, so a whole number of records opens.
    ///
    /// Without this row the check above would pass just as well if `open`
    /// refused every non-empty ledger, which would be a far worse defect wearing
    /// the same test's approval.
    #[test]
    fn a_ledger_whose_length_is_a_whole_number_of_records_still_opens() {
        let r = root("whole");
        {
            let mut store = Results::open(&r).expect("opens");
            for i in 1..=3 {
                store.append(&record(i)).expect("appends");
            }
        }
        let store = Results::open(&r).expect("a ledger with no orphan bytes reopens");
        assert_eq!(store.len().expect("measurable"), 3);
    }

    /// A WHOLE-STRIDE RECORD THAT WAS DAMAGED AFTER IT WAS WRITTEN IS REFUSED.
    ///
    /// # The case the stride check cannot reach
    ///
    /// `a_part_record_from_an_interrupted_write_is_refused_and_counted` catches
    /// a write that stopped PARTWAY through a record, because the file then
    /// measures as a fraction of a stride. It says nothing about a file that
    /// measures perfectly: a write that stopped exactly on a boundary, a bit
    /// flipped by a failing disk, or a byte put there by something that is not
    /// this program. Each of those leaves a whole number of strides.
    ///
    /// Nothing else would notice. `from_bytes` is infallible by construction —
    /// every byte pattern is a legal value of its type — so a damaged record
    /// parses cleanly and renders as a run that never happened. That is the same
    /// failure the 176/205 stride mistake would have caused, arriving by a
    /// different route.
    ///
    /// # Every field is tried, not just the first
    ///
    /// A seal computed over too short a window would still catch damage to the
    /// bytes it covers and miss everything past its end. Flipping a bit in the
    /// FIRST byte, the LAST byte before the seal, and one in the middle is what
    /// makes the window's width part of the assertion rather than an assumption.
    #[test]
    fn a_record_damaged_after_it_was_written_is_refused_and_not_parsed() {
        for spot in [0_usize, PAYLOAD_BYTES / 2, PAYLOAD_BYTES - 1] {
            let r = root(&format!("seal{spot}"));
            {
                let mut store = Results::open(&r).expect("opens");
                store.append(&record(1)).expect("appends");
                store.append(&record(2)).expect("appends");
            }
            // A single flipped bit inside record 1, the file length untouched.
            let path = Results::path(&r);
            let mut bytes = std::fs::read(&path).expect("the ledger reads");
            let at = HEADER_BYTES + STRIDE_BYTES + spot;
            *bytes.get_mut(at).expect(
                "the fixture wrote two whole records, so this offset is inside record 1",
            ) ^= 0b0000_0001;
            std::fs::write(&path, &bytes).expect("the damaged ledger writes");

            let mut store = Results::open(&r).expect("the LENGTH is still whole, so it opens");
            assert_eq!(
                store.len().expect("measurable"),
                2,
                "and it still measures as two whole records, which is precisely \
                 why the length check cannot catch this"
            );
            store
                .read(0)
                .expect("the undamaged record beside it is still readable");
            let why = store.read(1).expect_err("the damaged record is refused");
            assert!(
                why.contains("seal"),
                "the refusal must say what failed, or an operator reads it as a \
                 missing file: {why}"
            );
        }
    }

    /// The seal covers the payload and NOT itself.
    ///
    /// Without this row, a `seal_of` that hashed all `STRIDE_BYTES` — including
    /// the eight it is about to overwrite — would pass every test above, because
    /// writer and reader would agree with each other while agreeing about the
    /// wrong window. They would also both be wrong in the same way forever.
    #[test]
    fn the_seal_is_computed_over_the_payload_and_never_over_itself() {
        let mut raw = record(7).to_bytes();
        assert!(Record::seal_matches(&raw), "as written, it matches");
        // Damage only the seal slot: the payload is untouched, so a seal
        // computed over the payload alone must now DISAGREE.
        raw[PAYLOAD_BYTES] ^= 0b1000_0000;
        assert!(
            !Record::seal_matches(&raw),
            "a changed seal over an unchanged payload must not still match"
        );
        // And the width is exactly the eight bytes past the payload.
        assert_eq!(STRIDE_BYTES - PAYLOAD_BYTES, SEAL_BYTES);
    }

    /// TWO HANDLES ON ONE LEDGER LOSE NOTHING AND DUPLICATE NOTHING.
    ///
    /// # The failure this closes
    ///
    /// `cli`'s `LEDGER` mutex serialises threads of one process and says so in
    /// its own comment, ending *"a second `range-all` should not be started
    /// while one is running"*. The enforcement of that was the sentence itself,
    /// in a tree where two sessions and an HTTP server all reach one store.
    ///
    /// Without the file lock, `append`'s seek-then-write lets a second writer
    /// compute the same offset and land ON TOP of the first record. Both records
    /// are whole and both seals verify, so nothing downstream can tell that a
    /// run was destroyed — the count reads as one, and the loss leaves no trace.
    ///
    /// # Why two handles rather than two processes
    ///
    /// Two `Results` values hold two independent descriptors, two independent
    /// `seen` sets and two independent `scanned` watermarks, which is exactly
    /// the state two processes have. Spawning a real process would test the
    /// harness as much as the code, and could not run under `cargo test`'s own
    /// working directory without a fixture binary.
    ///
    /// The interleaving is deliberate: `b` opens BEFORE `a` writes, so `b`'s
    /// `seen` set is stale by construction. A version that only locked, without
    /// `absorb_new_records`, passes the no-loss half of this test and fails the
    /// no-duplicate half.
    #[test]
    fn a_second_writer_neither_overwrites_a_row_nor_duplicates_one() {
        let r = root("twowriters");
        let mut a = Results::open(&r).expect("the first handle opens");
        let mut b = Results::open(&r).expect("the second opens on the same file");

        a.append(&record(1)).expect("the first writer records");
        // `b` was opened before that write, so its `seen` cannot contain it and
        // its idea of the end of the file is one record short.
        b.append(&record(2)).expect("the second writer records");

        let mut check = Results::open(&r).expect("a fresh reader opens");
        assert_eq!(
            check.len().expect("measurable"),
            2,
            "both rows must survive: a lost row is the one failure an \
             append-only ledger exists to prevent, and it leaves no trace"
        );
        let ids = [
            check.read(0).expect("row 0 reads").identity,
            check.read(1).expect("row 1 reads").identity,
        ];
        assert!(ids.contains(&[1; 32]), "the first writer's row survived");
        assert!(ids.contains(&[2; 32]), "the second writer's row survived");

        // And the stale handle must still refuse a row the OTHER handle wrote,
        // which is what `absorb_new_records` is for.
        let why = b
            .append(&record(1))
            .expect_err("a duplicate is refused even though another handle wrote it");
        assert!(why.contains("already recorded"), "{why}");
        assert_eq!(
            check.len().expect("measurable"),
            2,
            "and the refusal appended nothing"
        );
    }

    /// EIGHT WRITERS AT ONCE LOSE NOTHING.
    ///
    /// # Why the test above is not enough, discovered by mutating both halves
    ///
    /// `a_second_writer_neither_overwrites_a_row_nor_duplicates_one` uses two
    /// handles but drives them in sequence, so it exercises the stale-`seen`
    /// re-scan and nothing else. Measured: deleting `absorb_new_records` fails
    /// it, and deleting the FILE LOCK does not. A lock with no test is a comment
    /// with a keyword in it.
    ///
    /// This one runs the writers genuinely at the same time. Each thread holds
    /// its own descriptor, its own `seen` and its own watermark — the state two
    /// processes have — and every one appends a distinct identity. Without the
    /// lock, two writers compute the same end offset between their seek and
    /// their write, and the second lands on top of the first: two whole records,
    /// two verifying seals, one run gone and no trace of it anywhere.
    ///
    /// # On flakiness
    ///
    /// A race that is not hit is not proof, and no thread test can promise the
    /// interleaving. What it can promise is the direction: this cannot fail when
    /// the lock is present, and it fails often when it is absent. Sixteen rounds
    /// of eight writers is enough contention to make the window likely without
    /// making the suite slow.
    #[test]
    fn eight_writers_at_once_all_survive() {
        const WRITERS: u8 = 8;
        const ROUNDS: u8 = 16;
        let r = root("racers");
        Results::open(&r).expect("the ledger exists before the writers start");

        let threads: Vec<_> = (0..WRITERS)
            .map(|w| {
                let dir = r.clone();
                std::thread::spawn(move || {
                    let mut mine = Results::open(&dir).expect("each writer opens its own handle");
                    for round in 0..ROUNDS {
                        // A distinct identity per writer per round, so nothing
                        // here is refused as a duplicate and every append MUST
                        // land. `w` and `round` both vary so no two collide.
                        let mut rec = record(1);
                        rec.identity = [0; 32];
                        rec.identity[0] = w;
                        rec.identity[1] = round;
                        mine.append(&rec).expect("every distinct run is recorded");
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().expect("no writer panicked");
        }

        let mut check = Results::open(&r).expect("reopens");
        let want = u64::from(WRITERS) * u64::from(ROUNDS);
        assert_eq!(
            check.len().expect("measurable"),
            want,
            "every write must survive: an overwrite destroys a run and leaves \
             the count looking correct"
        );
        // And each one is INTACT, not merely counted — an interleaved write
        // would leave a record whose bytes came from two different writers, and
        // only the seal can tell.
        let mut ids = std::collections::HashSet::new();
        for i in 0..want {
            let rec = check
                .read(i)
                .expect("every row verifies its seal, so no write was interleaved");
            assert!(ids.insert(rec.identity), "row {i} is a duplicate identity");
        }
    }

    /// A PATH THAT ACCEPTS BYTES AND DOES NOT KEEP THEM IS REFUSED.
    ///
    /// # The failure this closes
    ///
    /// Every call on the way in succeeds when `runs.bin` points at `/dev/null`:
    /// the create succeeds, the write succeeds, the flush succeeds. The file
    /// then measures zero, so the next open writes a header again and the ledger
    /// is eternally, silently empty. Measured before the fix: `cli results`
    /// printed `rows 0`, refused nothing, and exited 0 — and a sweep would have
    /// recorded hours of work into the void and reported success.
    ///
    /// `/dev/null` is the cheapest way to construct it, not the likely one. The
    /// same silence covers a filesystem with no room, a volume that unmounted
    /// under an open handle, and a file another process truncated between the
    /// create and the write. What they share is that WRITING SUCCEEDED and
    /// nothing was stored, which no error code reports.
    #[test]
    fn a_ledger_that_does_not_keep_what_it_is_given_is_refused() {
        let r = root("devnull");
        std::fs::create_dir_all(r.join("results")).expect("the directory is made");
        let path = Results::path(&r);
        let _ = std::fs::remove_file(&path);
        std::os::unix::fs::symlink("/dev/null", &path).expect("the symlink is made");

        let why = Results::open(&r).expect_err("a sink is not a ledger");
        assert!(
            why.contains("did not keep it"),
            "the refusal must say the bytes were accepted and lost, not merely \
             that something failed -- nothing failed, which is the problem: \
             {why}"
        );
    }

    #[test]
    fn the_same_identity_is_refused_rather_than_overwritten() {
        // §3 rule 5: same inputs, same outputs. A rerun has nothing new to say,
        // and overwriting would destroy the original's timestamp for no gain.
        let r = root("dup");
        let mut store = Results::open(&r).expect("opens");
        store.append(&record(9)).expect("the first is recorded");
        let why = store.append(&record(9)).expect_err("the second is refused");
        assert!(why.contains("already recorded"), "{why}");
        assert_eq!(
            store.len().expect("measurable"),
            1,
            "and nothing was appended"
        );
        // The duplicate check survives a reopen, because it is rebuilt from the
        // file rather than kept only in memory.
        drop(store);
        let mut store = Results::open(&r).expect("reopens");
        assert!(
            store.holds(&[9; 32]),
            "the identity is known after a reopen"
        );
        assert!(store.append(&record(9)).is_err(), "and still refused");
    }

    #[test]
    fn a_file_that_is_not_this_format_refuses_rather_than_being_appended_to() {
        let r = root("alien");
        std::fs::create_dir_all(r.join("results")).expect("the directory");
        std::fs::write(Results::path(&r), b"NOTBRUTEXand some more bytes here")
            .expect("an alien file");
        let why = Results::open(&r).expect_err("refuses");
        assert!(
            why.contains("not a brutex results file"),
            "the refusal must name what it found: {why}"
        );
    }

    #[test]
    fn a_version_this_build_does_not_write_refuses() {
        // §4: a new field is a new file version at its own stride. A build that
        // appended its own stride to another version's file would interleave two
        // layouts in one addressable space.
        let r = root("version");
        std::fs::create_dir_all(r.join("results")).expect("the directory");
        let mut header = [0_u8; HEADER_BYTES];
        header[..8].copy_from_slice(b"BRUTEXRS");
        header[8..12].copy_from_slice(&99_u32.to_le_bytes());
        std::fs::write(Results::path(&r), header).expect("a future file");
        let why = Results::open(&r).expect_err("refuses");
        assert!(why.contains("version 99"), "{why}");
    }

    #[test]
    fn an_index_past_the_end_is_refused_and_names_the_count() {
        let r = root("oob");
        let mut store = Results::open(&r).expect("opens");
        store.append(&record(1)).expect("appends");
        let why = store.read(7).expect_err("refuses");
        assert!(why.contains("this file holds 1"), "{why}");
    }

    #[test]
    fn a_text_field_longer_than_its_slot_is_truncated_and_never_bleeds() {
        // Truncation is safe HERE because the identity names the run and these
        // fields are for a human reading a listing. What must never happen is a
        // write past the slot, which would corrupt the next field.
        let long = field("an-instrument-name-far-longer-than-sixteen-bytes");
        assert_eq!(long.len(), 16, "the slot is exactly its declared width");
        assert_eq!(read_field(&long), "an-instrument-na");
    }

    /// A version-2 ledger is READ, and its records come back whole.
    ///
    /// # The loss this prevents, and it was hours from happening
    ///
    /// Version 3 first shipped refusing any file whose header did not say 3.
    /// Two sweeps had been running for nine and five hours, both writing
    /// version 2 to the ledger they had opened before the new binary existed.
    /// The moment they finished, every figure they had produced would have been
    /// unreadable -- not because anything was wrong with the data, but because
    /// a version check treated "cannot append" and "cannot open" as one thing.
    ///
    /// `CLAUDE.md` §3 rule 8 protects a stored format's history. Refusing to
    /// READ that history is not what it asks for.
    #[test]
    fn a_version_two_ledger_is_read_and_its_records_come_back_whole() {
        let root = std::env::temp_dir().join("brutex-v2-read-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("a temp root");
        let path = Results::path(&root);

        let original = record(9);
        std::fs::write(&path, v2_file_holding(&original)).expect("the v2 file writes");

        let mut store = Results::open(&root).expect("a version-2 ledger must OPEN");
        assert_eq!(store.len().expect("a length"), 1, "at version 2's stride");

        let back = store.read(0).expect("and its record must READ");
        assert_eq!(back.identity, original.identity, "identity survived");
        assert_eq!(back.trades, original.trades, "trade count survived");
        assert_eq!(
            back.pessimistic, original.pessimistic,
            "the money survived, which is the whole point"
        );
        assert_eq!(
            back.exit_rungs, original.exit_rungs,
            "the LAST version-2 field survived -- if this passes, every offset \
             before it did too"
        );
        assert_eq!(
            back.mask_words, [0; 6],
            "and the field version 2 never had reads as absent, which the \
             report renders as `no combination was recorded`"
        );
    }

    /// A version-2 ledger is never APPENDED to, and says why.
    ///
    /// Appending a 261-byte record to a file addressed every 213 bytes mutates
    /// the format in place, which §3 rule 8 forbids, and every record after it
    /// would decode at the wrong offsets -- parsing cleanly, because every byte
    /// pattern here is a legal record.
    #[test]
    fn a_version_two_ledger_is_never_appended_to() {
        let root = std::env::temp_dir().join("brutex-v2-append-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("a temp root");
        std::fs::write(Results::path(&root), v2_file_holding(&record(4)))
            .expect("the v2 file writes");

        let mut store = Results::open(&root).expect("it opens for reading");
        let refused = store
            .append(&record(5))
            .expect_err("a version-2 ledger must refuse an append");

        assert!(
            refused.contains("version 2"),
            "the refusal must name the version it found: {refused}"
        );
        assert!(
            refused.contains("READ"),
            "and must say the data is not lost, or an operator deletes it: \
             {refused}"
        );
        let after = std::fs::metadata(Results::path(&root))
            .expect("the file still exists")
            .len();
        assert_eq!(
            after,
            HEADER + STRIDE_V2,
            "and NOTHING may have been written: {after} bytes"
        );
    }

    /// A version this build has never written is refused rather than guessed at.
    #[test]
    fn a_version_from_the_future_is_refused_rather_than_guessed_at() {
        let root = std::env::temp_dir().join("brutex-v9-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("a temp root");
        let mut bytes = v2_file_holding(&record(1));
        // Stamp a version whose field offsets this build cannot know.
        bytes.splice(8..12, 9_u32.to_le_bytes());
        std::fs::write(Results::path(&root), bytes).expect("the file writes");

        let refused = Results::open(&root).expect_err("an unknown version refuses");
        assert!(
            refused.contains("version 9"),
            "the refusal names what it found: {refused}"
        );
        assert!(
            refused.contains("will not guess"),
            "and says it is declining to guess offsets, which is the reason: \
             {refused}"
        );
    }

    /// A version-2 file holding exactly one record, as bytes.
    ///
    /// Built from a version-3 record by keeping its payload up to the mask and
    /// sealing THAT length -- which is precisely what a version-2 writer did,
    /// and is only correct because the mask was APPENDED rather than inserted.
    /// The compile-time assertion beside `PAYLOAD_BYTES_V2` is what holds that.
    fn v2_file_holding(r: &Record) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION_V2.to_le_bytes());
        out.extend_from_slice(&[0_u8; 4]);
        let wide = r.to_bytes();
        let payload = wide.get(..PAYLOAD_BYTES_V2).expect("the v2 prefix");
        out.extend_from_slice(payload);
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(payload);
        let full = hasher.finalize();
        out.extend_from_slice(full.get(..SEAL_BYTES).expect("eight seal bytes"));
        out
    }
}
