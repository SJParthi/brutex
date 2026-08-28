//! The two-slot header, and the commit that is one write of one unit.
//!
//! # What was wrong
//!
//! `docs/05-decisions.md` D-0004 makes the commit counter the authority and
//! publishes it last, so a torn *record* is unobservable. That argument is
//! sound and it survives here unchanged. What it does not cover is a torn
//! *header*: publishing a commit meant writing `n_valid`, then
//! `last_ts_micros`, then `header_crc` as three separate stores. A crash
//! between any two of them left a header whose checksum did not match its own
//! counter — and a header that fails its checksum condemns the **whole file**,
//! not the tail. The counter made the last record safe and the header unsafe.
//!
//! # The shape that fixes it
//!
//! A header **slot** is 64 bytes carrying every field *and* the checksum over
//! them. A file holds [`Layout::slot_count`] slots at
//! [`crate::format::SLOT_STRIDE`] spacing, and commit *g* is written to slot
//! `g % slot_count`. Consecutive commits therefore never write the same slot,
//! and a reader takes the valid slot with the highest generation **that the
//! file's bytes actually support**.
//!
//! ```text
//! byte 0          16384          32768
//!   ├── slot 0 ─────┼── slot 1 ────┤   generation 4 → slot 0
//!    generation 4     generation 3     generation 5 → slot 1
//! ```
//!
//! A crash can land in exactly one of three places, and none of them is a
//! self-inconsistent header:
//!
//! | Crash point | What survives | What a reader sees |
//! |---|---|---|
//! | before the slot write | both slots whole | generation *g−1*: the records committed before |
//! | during the slot write | the *other* slot whole | generation *g−1*, because a partial slot fails its own checksum |
//! | after the slot write | both slots whole | generation *g*: the new records |
//!
//! The reader never sees a count that was not committed — only *g−1* or *g*.
//! `store::fault::a_torn_header_commit_never_reports_an_uncommitted_count`
//! writes every one of the 65 possible prefixes of a commit image and asserts
//! exactly that.
//!
//! The fourth row of that table is the one that was missing: a slot that is
//! **whole but unsupported**, because the records it counts never reached the
//! disk. [`Header::read_region`] falls back to the previous generation for
//! that case too, instead of condemning the file — see
//! `store::fault::kill_between_write_and_commit`.
//!
//! # Why not the alternatives
//!
//! **One aligned word holding counter, timestamp and checksum.** Arithmetically
//! impossible: those are 8 + 8 + 4 bytes and a machine word is 8. Making them
//! fit means truncating the timestamp, which is a fabrication in the one field
//! a range reject depends on.
//!
//! **The counter as the only authority, everything else derived.** It still
//! needs one 8-byte store to be atomic across a power loss, which is a
//! property of the device, not of this program — "in practice atomic" is not a
//! measurement. It also derives `last_ts_micros` by reading record
//! `n_valid − 1`, which cannot be trusted until its block checksum is
//! verified, which needs a header. A dependency loop on the open path.
//!
//! # The three assumptions, named rather than assumed
//!
//! An earlier version of this module claimed two slots assume "only that a
//! write to slot A cannot damage slot B … they are disjoint byte ranges, so
//! that is true on any device". **That was false.** Storage does not fail at
//! byte granularity; it fails at a block or a page, and two slots 64 bytes
//! apart share one of those. The three assumptions the crash table really
//! rests on are:
//!
//! 1. **A write to one slot cannot damage another.** Now structural:
//!    [`crate::format::SLOT_STRIDE`] is 16384 bytes, which is at least the
//!    device block *and* the page size measured on both hosts this repository
//!    builds on. Residual limit: a failure coarser than 16384 bytes takes both
//!    slots, and no arrangement inside one file survives that.
//! 2. **The appended records are durable before the slot write.** Now carried
//!    by the type: [`Commit::durable_through`] is the byte offset a writer
//!    must have flushed before it issues the slot write. This crate performs
//!    no I/O and cannot issue the barrier itself, so it states the offset
//!    instead of leaving the requirement in prose. When the requirement is
//!    broken anyway, [`Header::read_region`] falls back to the previous
//!    generation rather than handing back uncommitted bytes, and
//!    [`crate::block`] is what detects the records that were lost.
//! 3. **Exactly one writer per file.** Two writers reading the same generation
//!    produce the same slot at the same offset and the same record range.
//!    Nothing in this crate can enforce that — an exclusive lock is I/O — so
//!    it is a documented precondition of [`Header::commit`], the lock file has
//!    a name in the path type ([`crate::path::FileKind::Lock`]), and the gap
//!    is reported as a limit rather than implied away.
//!
//! # It is still a read-only mapping plus `pwrite`
//!
//! Nothing here writes. [`Header::commit`] returns a [`Commit`] — one offset
//! and one 64-byte buffer — which a writer hands to a single positional write.
//! A writable mapping remains banned: it raises `SIGBUS` on a full disk, and a
//! signal cannot be caught. **There is no API in this module that can update
//! two header fields in two writes**, because a `Commit` carries one offset
//! and one buffer and there is no other way to produce header bytes.
//!
//! [`Header::decode`] takes its own 64-byte copy of a slot before it checksums
//! anything, so a caller may hand it a `MAP_SHARED` mapping that a writer is
//! concurrently `pwrite`-ing: the checksum covers exactly the bytes the
//! returned `Header` is built from, and a copy that caught a write mid-flight
//! fails that checksum rather than returning half of each.

use crate::crc::crc32c_split;
use crate::format::{FLAG_CHECKSUMS, FormatError, MAGIC_FAMILY, MAX_SLOTS, SLOT_LEN, SLOT_STRIDE};
use crate::layout::Layout;

/// Byte offset of `magic` within a slot.
const OFF_MAGIC: usize = 0;
/// Byte offset of `format_version` within a slot.
const OFF_VERSION: usize = 8;
/// Byte offset of `record_stride` within a slot.
const OFF_STRIDE: usize = 10;
/// Byte offset of `flags` within a slot.
const OFF_FLAGS: usize = 12;
/// Byte offset of `generation` within a slot.
const OFF_GENERATION: usize = 16;
/// Byte offset of `n_valid` within a slot.
const OFF_N_VALID: usize = 24;
/// Byte offset of `first_ts_micros` within a slot.
const OFF_FIRST_TS: usize = 32;
/// Byte offset of `last_ts_micros` within a slot.
const OFF_LAST_TS: usize = 40;
/// Byte offset of `symbol_id` within a slot.
const OFF_SYMBOL_ID: usize = 48;
/// Byte offset of `timeframe_secs` within a slot.
const OFF_TIMEFRAME: usize = 52;
/// Byte offset of the slot checksum.
const OFF_CRC: usize = 56;
/// Byte offset of the reserved tail, which is zero and stays zero.
const OFF_RESERVED: usize = 60;

const _: () = assert!(OFF_RESERVED < SLOT_LEN);
const _: () = assert!(OFF_CRC + 4 == OFF_RESERVED);

/// [`crate::format::SLOT_STRIDE`] as a `usize`, for slicing a region.
///
/// Written as a literal in both widths rather than converted, so there is no
/// cast to deny and no fallible conversion whose failure arm no test could
/// ever reach. The assertion is what keeps the two honest.
const SLOT_STRIDE_LEN: usize = 16_384;
const _: () = assert!(SLOT_STRIDE == 16_384 && SLOT_STRIDE_LEN == 16_384);

/// [`Layout::CURRENT`]'s record stride, in the width the slot stores it.
///
/// Written as a literal in both widths for the same reason. A future
/// `CURRENT` with a different stride is a compile error here.
const CURRENT_STRIDE: u16 = 56;
const _: () = assert!(CURRENT_STRIDE == 56 && Layout::CURRENT.record_stride() == 56);

/// One header slot's fields.
///
/// `n_valid` is the commit counter — `docs/05-decisions.md` D-0004. A reader
/// treats records `0..n_valid` as the whole file, so a half-written record at
/// the tail is not merely unlikely, it is **unobservable**. `generation`
/// extends that guarantee to the header itself: it says which slot is the
/// newest, so a half-written *header* is unobservable too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Header {
    /// Format version. Selects the layout; never assumed.
    pub format_version: u16,
    /// Bytes per record, as this file declares them.
    pub record_stride: u16,
    /// Bit 0: block checksums present.
    pub flags: u32,
    /// Which commit this slot holds. Higher wins.
    pub generation: u64,
    /// The commit counter: how many records are readable.
    pub n_valid: u64,
    /// Timestamp of record 0, for a cheap range reject.
    ///
    /// Meaningful only when `n_valid > 0`. [`Header::advance`] sets it from
    /// the first batch appended to an empty file and never moves it again, so
    /// a range reject over `[first_ts_micros, last_ts_micros]` describes the
    /// records that are actually there.
    pub first_ts_micros: i64,
    /// Timestamp of record `n_valid - 1`. Meaningful only when `n_valid > 0`.
    pub last_ts_micros: i64,
    /// Resolved from the path; a cross-check, never the index.
    pub symbol_id: u32,
    /// Seconds per bar. 60 for one minute.
    pub timeframe_secs: u32,
}

/// One header write: one offset, one buffer, one `pwrite`.
///
/// The type is the guarantee. A commit cannot be expressed as several field
/// updates because there is nowhere to put the second one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commit {
    /// Which slot this belongs in — `generation % slot_count`.
    pub slot: u64,
    /// The byte offset to write at.
    pub offset: u64,
    /// Exactly the bytes to write, checksum included.
    pub bytes: [u8; SLOT_LEN],
    /// The byte offset through which record data must already be durable.
    ///
    /// `header_len + n_valid · stride` — the end of the last record this
    /// commit publishes. Issuing the slot write before the bytes below this
    /// offset are on stable storage publishes a counter over data that may not
    /// exist; the header page is re-dirtied on every commit and is therefore
    /// the hottest writeback candidate in the file, so the reordering is the
    /// likely case rather than the exotic one.
    ///
    /// This crate performs no I/O and cannot issue the barrier. It states the
    /// offset so a writer cannot claim it did not know which one to flush.
    pub durable_through: u64,
    /// The header state this commit publishes.
    pub header: Header,
}

impl Header {
    /// A fresh header for an empty file: generation 0, no records.
    ///
    /// The caller writes [`Header::commit`] into a zero-filled header region.
    /// Every other slot stays zero, which fails the magic check and is
    /// therefore not a candidate — an empty slot and a corrupt one are the
    /// same thing to a reader, and both are safe.
    #[must_use]
    pub const fn genesis(symbol_id: u32, timeframe_secs: u32, flags: u32) -> Self {
        Self::genesis_at(Layout::CURRENT, symbol_id, timeframe_secs, flags)
    }

    /// The first header of a file at a GIVEN geometry.
    ///
    /// # Why the geometry is a parameter
    ///
    /// [`Self::genesis`] wrote `Layout::CURRENT` and its stride, which is right
    /// for a `.bar` and silently wrong for its `.ovl` sibling: the sidecar's
    /// records are 24 bytes and its version is 9, so a file created at the bar's
    /// geometry accepts a 24-byte batch at 56-byte offsets. The header would
    /// validate, the CRC would pass — the bytes written are the bytes read —
    /// and every field afterwards would come from the wrong place.
    ///
    /// Naming the layout at creation is what makes that a compile-time choice
    /// rather than a default nobody revisits.
    #[must_use]
    pub const fn genesis_at(
        layout: Layout,
        symbol_id: u32,
        timeframe_secs: u32,
        flags: u32,
    ) -> Self {
        // THE STRIDE COMES FROM THE LAYOUT, not from a constant beside it, so
        // the two cannot disagree about one file.
        #[allow(clippy::cast_possible_truncation)]
        let record_stride = layout.record_stride() as u16;
        Self {
            format_version: layout.version(),
            record_stride,
            flags,
            generation: 0,
            n_valid: 0,
            first_ts_micros: 0,
            last_ts_micros: 0,
            symbol_id,
            timeframe_secs,
        }
    }

    /// Whether this file carries per-block checksums.
    ///
    /// A file without them cannot have a block verified at all, and
    /// [`crate::block::verify`] says so rather than returning "fine".
    #[must_use]
    pub const fn checksums_present(&self) -> bool {
        self.flags & FLAG_CHECKSUMS != 0
    }

    /// The header that publishing `appended` more records produces.
    ///
    /// Advances the generation and the counter together, because they are
    /// written together, and carries the batch's timestamps into the range the
    /// header advertises. `first_ts_micros` is taken from the **first** batch
    /// appended to an empty file and never moved afterwards; every later batch
    /// only moves `last_ts_micros`. Before this, no method in this crate set
    /// `first_ts_micros` at all, so every header it could commit claimed
    /// record 0 sat at the Unix epoch and a range reject built on it accepted
    /// every file that has ever existed.
    ///
    /// When `appended` is zero there is no batch to describe: the counter and
    /// both timestamps are carried forward unchanged and the two timestamp
    /// arguments are not read. That is a re-publish of the same state under a
    /// new generation, which is idempotent and safe.
    ///
    /// # Errors
    ///
    /// [`FormatError::GenerationExhausted`] or [`FormatError::CounterOverflow`]
    /// — both refusals, never wraps. A wrapped generation makes the *oldest*
    /// slot win and silently un-commits records.
    ///
    /// [`FormatError::TimestampsOutOfOrder`] when the batch's own timestamps
    /// run backwards, or when it begins at or before the last timestamp
    /// already committed. Bars are append-only in time, and a re-pull landing
    /// on top of newer data is well-formed bytes that no checksum can catch
    /// afterwards.
    pub const fn advance(
        &self,
        appended: u64,
        first_ts_micros: i64,
        last_ts_micros: i64,
    ) -> Result<Self, FormatError> {
        let (generation, n_valid) = match (
            self.generation.checked_add(1),
            self.n_valid.checked_add(appended),
        ) {
            (None, _) => return Err(FormatError::GenerationExhausted),
            (Some(_), None) => return Err(FormatError::CounterOverflow),
            (Some(generation), Some(n_valid)) => (generation, n_valid),
        };
        if appended == 0 {
            return Ok(Self {
                generation,
                ..*self
            });
        }
        if last_ts_micros < first_ts_micros {
            return Err(FormatError::TimestampsOutOfOrder {
                previous: first_ts_micros,
                next: last_ts_micros,
            });
        }
        if self.n_valid > 0 && first_ts_micros <= self.last_ts_micros {
            return Err(FormatError::TimestampsOutOfOrder {
                previous: self.last_ts_micros,
                next: first_ts_micros,
            });
        }
        let first = if self.n_valid == 0 {
            first_ts_micros
        } else {
            self.first_ts_micros
        };
        Ok(Self {
            generation,
            n_valid,
            first_ts_micros: first,
            last_ts_micros,
            ..*self
        })
    }

    /// The single write that publishes this header.
    ///
    /// # Preconditions the caller owns
    ///
    /// 1. **One writer per file.** Two writers that read the same generation
    ///    produce the same slot, the same offset and the same record range;
    ///    whichever lands last wins, and a crash during the second destroys
    ///    the slot that held the only copy of the previous commit. Take an
    ///    exclusive lock on [`crate::path::FileKind::Lock`] first. This crate
    ///    performs no I/O and cannot check it.
    /// 2. **Flush first.** Every byte below [`Commit::durable_through`] must
    ///    be on stable storage before this write is issued.
    ///
    /// # Errors
    ///
    /// [`FormatError::UnknownVersion`] or [`FormatError::RetiredVersion`] if
    /// this header names a version this build cannot write,
    /// [`FormatError::StrideMismatch`] if it names a stride its own version
    /// does not define, or [`FormatError::OffsetOverflow`] if the counter puts
    /// the end of the data past `u64`.
    pub fn commit(&self) -> Result<Commit, FormatError> {
        self.commit_image()
            .inspect_err(|&refusal| note_commit_refused(self, refusal))
    }

    /// [`Header::commit`]'s body, split so that its three refusals — the
    /// version, the stride and the offset — reach **one** emit site instead of
    /// three. Splitting for the event rather than repeating the event is what
    /// keeps a later fourth refusal from being the one nobody logged.
    fn commit_image(&self) -> Result<Commit, FormatError> {
        let layout = Layout::for_version(self.format_version)?;
        if u64::from(self.record_stride) != layout.record_stride() {
            return Err(FormatError::StrideMismatch(self.record_stride));
        }
        Ok(Commit {
            slot: self.generation % layout.slot_count(),
            offset: layout.slot_offset(self.generation),
            bytes: self.image(layout.magic()),
            durable_through: layout.offset_of(self.n_valid)?,
            header: *self,
        })
    }

    /// Checks this header against a known file length.
    ///
    /// # Errors
    ///
    /// [`FormatError::StrideMismatch`] for a stride its version does not
    /// define, [`FormatError::TooShortForHeader`] for a file that cannot hold
    /// the header region, [`FormatError::CounterExceedsFile`] for a counter
    /// claiming more records than the bytes can contain — trusting the counter
    /// over the length reads past the end and returns whatever was there — or
    /// [`FormatError::TimestampsOutOfOrder`] for a non-empty file whose
    /// advertised range runs backwards.
    pub fn validate(&self, layout: Layout, file_len: u64) -> Result<(), FormatError> {
        if u64::from(self.record_stride) != layout.record_stride() {
            return Err(FormatError::StrideMismatch(self.record_stride));
        }
        if file_len < layout.header_len() {
            return Err(FormatError::TooShortForHeader);
        }
        if self.n_valid > layout.capacity_for(file_len) {
            return Err(FormatError::CounterExceedsFile);
        }
        if self.n_valid > 0 && self.last_ts_micros < self.first_ts_micros {
            return Err(FormatError::TimestampsOutOfOrder {
                previous: self.first_ts_micros,
                next: self.last_ts_micros,
            });
        }
        Ok(())
    }

    /// Decodes one slot, checksum and all.
    ///
    /// The slot is copied into a 64-byte array **once**, before anything is
    /// checked, and every field is then read from that copy. That is what
    /// makes it safe over a `MAP_SHARED` mapping a writer is `pwrite`-ing: the
    /// checksum verifies exactly the bytes the returned `Header` carries. Were
    /// the buffer re-read after the checksum passed, a write landing in
    /// between would produce a header no checksum ever covered — and with
    /// `#![forbid(unsafe_code)]` there is no fence available to order plain
    /// loads, so the copy is the fix rather than a mitigation.
    ///
    /// # Errors
    ///
    /// [`FormatError::SlotTooShort`], [`FormatError::NotABarFile`],
    /// [`FormatError::UnknownVersion`], [`FormatError::RetiredVersion`],
    /// [`FormatError::MagicVersionMismatch`], [`FormatError::SlotChecksum`] or
    /// [`FormatError::StrideMismatch`] — checked in that order, so the most
    /// basic disagreement is the one reported.
    pub fn decode(slot: &[u8]) -> Result<Self, FormatError> {
        Self::decode_parts(slot).map(|(header, _)| header)
    }

    /// [`Header::decode`], keeping the layout it already resolved.
    ///
    /// [`Header::read_region`] needs both, and re-resolving the version would
    /// add an error path no test could reach — the version was accepted three
    /// lines earlier.
    fn decode_parts(slot: &[u8]) -> Result<(Self, Layout), FormatError> {
        if slot.len() < SLOT_LEN {
            return Err(FormatError::SlotTooShort { len: slot.len() });
        }
        // The one observation of the caller's bytes. Everything below reads
        // this array; nothing below touches `slot` again.
        let mut image = [0u8; SLOT_LEN];
        for (dst, src) in image.iter_mut().zip(slot.iter()) {
            *dst = *src;
        }

        let magic: [u8; 8] = le_bytes(&image, OFF_MAGIC);
        if magic
            .iter()
            .take(MAGIC_FAMILY.len())
            .ne(MAGIC_FAMILY.iter())
        {
            return Err(FormatError::NotABarFile);
        }
        let format_version = u16::from_le_bytes(le_bytes(&image, OFF_VERSION));
        let layout = Layout::for_version(format_version)?;
        if magic != layout.magic() {
            return Err(FormatError::MagicVersionMismatch(format_version));
        }
        let stored = u32::from_le_bytes(le_bytes(&image, OFF_CRC));
        let (head, tail) = covered(&image);
        let computed = crc32c_split(&head, &tail);
        if stored != computed {
            return Err(FormatError::SlotChecksum { stored, computed });
        }
        let record_stride = u16::from_le_bytes(le_bytes(&image, OFF_STRIDE));
        if u64::from(record_stride) != layout.record_stride() {
            return Err(FormatError::StrideMismatch(record_stride));
        }
        Ok((
            Self {
                format_version,
                record_stride,
                flags: u32::from_le_bytes(le_bytes(&image, OFF_FLAGS)),
                generation: u64::from_le_bytes(le_bytes(&image, OFF_GENERATION)),
                n_valid: u64::from_le_bytes(le_bytes(&image, OFF_N_VALID)),
                first_ts_micros: i64::from_le_bytes(le_bytes(&image, OFF_FIRST_TS)),
                last_ts_micros: i64::from_le_bytes(le_bytes(&image, OFF_LAST_TS)),
                symbol_id: u32::from_le_bytes(le_bytes(&image, OFF_SYMBOL_ID)),
                timeframe_secs: u32::from_le_bytes(le_bytes(&image, OFF_TIMEFRAME)),
            },
            layout,
        ))
    }

    /// Reads the whole header region and returns the committed state.
    ///
    /// Each of the first [`crate::format::MAX_SLOTS`] slot positions is
    /// decoded independently — each carries its own magic, version and
    /// checksum — and the valid one with the highest generation that the
    /// file's length supports wins. A slot that is empty, stale, half-written,
    /// corrupt or in the wrong position is simply not a candidate.
    ///
    /// Two things it deliberately does **not** do:
    ///
    /// * It does not scan past [`crate::format::MAX_SLOTS`] positions. The
    ///   region a caller passes may be a whole read-only mapping, and every
    ///   aligned run of record bytes would otherwise audition as a header.
    /// * It does not condemn the file when the newest slot fails its check
    ///   against the file length. That is the crash where the header became
    ///   durable before the records it counts, and the previous generation is
    ///   sitting intact in the other slot; returning an error there loses a
    ///   month of bars to recover a tail.
    ///
    /// # Errors
    ///
    /// [`FormatError::HeaderRegionTooShort`] when the region is smaller than
    /// the version's header region — a short region would otherwise return
    /// whichever older commit happened to fit, silently losing every record
    /// committed since. [`FormatError::NoValidHeader`] when no slot decodes
    /// and none gave a specific reason: every copy of the header is damaged,
    /// and anything else this function could return would be a guess. When a
    /// slot did give a specific reason — an unknown or retired version, a
    /// stride disagreement, a failed checksum, or a commit found in the wrong
    /// slot — that reason is reported instead, because "the header is
    /// unreadable" is a false diagnosis of an intact file from another
    /// version. When slots decode but none survives [`Header::validate`], the
    /// newest one's refusal is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use store::{format::FormatError, header::Header};
    /// // An empty file: a zeroed header region, then the genesis commit.
    /// let mut region = vec![0u8; 32_768];
    /// let genesis = Header::genesis(1, 60, 0).commit()?;
    /// assert_eq!(genesis.offset, 0);
    /// assert_eq!(genesis.durable_through, 32_768);
    /// region
    ///     .iter_mut()
    ///     .zip(genesis.bytes)
    ///     .for_each(|(dst, src)| *dst = src);
    ///
    /// let read = Header::read_region(&region, 32_768)?;
    /// assert_eq!(read.n_valid, 0);
    /// assert_eq!(read.generation, 0);
    /// # Ok::<(), FormatError>(())
    /// ```
    pub fn read_region(region: &[u8], file_len: u64) -> Result<Self, FormatError> {
        let (header, newest, refused) = committed(region, file_len)
            .inspect_err(|&refusal| note_header_unreadable(region, file_len, refusal))?;
        if header.generation != newest {
            note_header_fell_back(&header, newest, file_len, refused);
        }
        Ok(header)
    }

    /// The 64-byte slot image for this header, checksum computed.
    fn image(&self, magic: [u8; 8]) -> [u8; SLOT_LEN] {
        let mut out = [0u8; SLOT_LEN];
        let body = magic
            .into_iter()
            .chain(self.format_version.to_le_bytes())
            .chain(self.record_stride.to_le_bytes())
            .chain(self.flags.to_le_bytes())
            .chain(self.generation.to_le_bytes())
            .chain(self.n_valid.to_le_bytes())
            .chain(self.first_ts_micros.to_le_bytes())
            .chain(self.last_ts_micros.to_le_bytes())
            .chain(self.symbol_id.to_le_bytes())
            .chain(self.timeframe_secs.to_le_bytes());
        for (dst, src) in out.iter_mut().zip(body) {
            *dst = src;
        }
        let (head, tail) = covered(&out);
        let crc = crc32c_split(&head, &tail);
        for (dst, src) in out.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
            *dst = src;
        }
        out
    }
}

/// The committed header, the newest generation any slot claimed, and the
/// refusal that stood between the two.
///
/// This is [`Header::read_region`]'s search, moved out of it whole and
/// unchanged. It is split off for one reason: the search can end in three
/// different places — no candidate at all, a region too short for the version,
/// and every candidate refused in turn — and three `return Err` statements
/// would need three copies of the same emit. One of them would eventually be
/// added without the other two, and the refusal nobody logged is exactly the
/// one an operator would be looking for.
///
/// The second element is the generation of the newest slot that *decoded*,
/// which is not always the generation returned: when the newest commit fails
/// [`Header::validate`], an older one is handed back on purpose. The third is
/// the refusal that rejected the newer commit, so the caller can say **why** it
/// walked back rather than only that it did.
fn committed(region: &[u8], file_len: u64) -> Result<(Header, u64, FormatError), FormatError> {
    let (header, layout) = best_candidate(region, None)?;

    let slots = whole_slots(region);
    if slots < layout.slot_count() {
        return Err(FormatError::HeaderRegionTooShort {
            slots,
            need: layout.slot_count(),
        });
    }

    // Newest first, at most one candidate per slot position. Bounded by
    // the family slot count rather than by the generation strictly
    // decreasing: a loop that leaned on the comparison would *hang* rather
    // than fail if that comparison ever stopped being strict, and a hang
    // is the one failure a test suite cannot report.
    let claimed = header.generation;
    let mut newest = Some((header, layout));
    let mut refusal = FormatError::NoValidHeader;
    for _ in 0..MAX_SLOTS {
        let Some((candidate, geometry)) = newest else {
            return Err(refusal);
        };
        match candidate.validate(geometry, file_len) {
            Ok(()) => return Ok((candidate, claimed, refusal)),
            Err(refused) => refusal = refused,
        }
        // The "no older candidate" error is dropped on purpose: the
        // refusal from the newest slot that decoded says more about the
        // file than "nothing else was there".
        newest = best_candidate(region, Some(candidate.generation)).ok();
    }
    Err(refusal)
}

/// A header region that yielded no usable commit, on the rolling log.
///
/// # What was invisible
///
/// A file whose every header slot is damaged returned a [`FormatError`] to its
/// caller and nothing else happened. The caller turns it into one refusal about
/// one file, and a store that had lost a header looked from the outside exactly
/// like a month that was never pulled — same empty answer, different cause, no
/// way to tell them apart afterwards. An operator can now read which refusal
/// the header search ended on, how many whole slots the region even held, and
/// the file length that was checked against, without a hex dump.
///
/// # What it deliberately does not carry
///
/// **The path**, because this module has none: it is handed a byte region and a
/// length, and inventing a name for the file would be a fabrication of the kind
/// `CLAUDE.md` §3 rule 1 forbids. The path belongs to the caller in
/// `crates/store/src/file.rs`, which knows it and already names it in
/// `store.append`.
///
/// # Why `Error`, and why this is not a per-row emit
///
/// It fires at most **once per file open**, and only on the refusal — the
/// ordinary successful read emits nothing at all, because that path runs once
/// per bar file in a sweep and the write side is already covered by
/// `store.append committed`.
fn note_header_unreadable(region: &[u8], file_len: u64, refusal: FormatError) {
    let why = refusal.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("store.header", "no committed header")
            .with("slots", telemetry::Value::Uint(whole_slots(region)))
            .with("region_bytes", telemetry::Value::count(region.len()))
            .with("file_len", telemetry::Value::Uint(file_len))
            .with("why", telemetry::Value::Str(&why)),
    );
}

/// A read that walked back to an older commit, on the rolling log.
///
/// # What was invisible
///
/// This is the fourth row of the table at the top of this module: the header
/// became durable before the records it counts, so the newest slot is whole and
/// **unsupported**, and [`Header::read_region`] hands back the previous
/// generation rather than condemning the file. That recovery is right and it
/// was completely silent. A file that had lost its last batch read back as a
/// file that never had one, and the only symptom was a bar count nobody held a
/// prior for. The question an operator could not answer — *did this file lose a
/// commit, or was that batch never pulled?* — is now one line: the generation
/// that was used, the generation that was rejected, and the refusal that
/// rejected it.
///
/// # Why `Warn` and not `Error`
///
/// Nothing failed. Every bar returned is real and was committed. It is the
/// **newest** commit that is gone, which is a fact worth a line and not worth a
/// refusal — and `CLAUDE.md` §4 bans a fallback that hides a failure, which is
/// precisely what this fallback was until now.
fn note_header_fell_back(header: &Header, newest: u64, file_len: u64, refusal: FormatError) {
    let why = refusal.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.header", "fell back to an older generation")
            .with("generation", telemetry::Value::Uint(header.generation))
            .with("rejected", telemetry::Value::Uint(newest))
            .with("n_valid", telemetry::Value::Uint(header.n_valid))
            .with(
                "symbol_id",
                telemetry::Value::Uint(u64::from(header.symbol_id)),
            )
            .with("file_len", telemetry::Value::Uint(file_len))
            .with("why", telemetry::Value::Str(&why)),
    );
}

/// A commit that could not be built, on the rolling log.
///
/// # What was invisible
///
/// [`Header::commit`] refuses a version this build cannot write, a stride the
/// version does not define, or a counter whose data end is past `u64`. Each of
/// those aborts an append, and the header state that produced it lived only in
/// memory and was dropped with the error — so the numbers that caused the
/// refusal did not survive it. The refusal reached a caller as one variant with
/// one number in it; the *state* that produced it reached nobody. An operator
/// can now see the version, the stride and the counter the writer was actually
/// holding, which is what separates "this build is older than the file" from
/// "this header was assembled wrong".
///
/// # Why `Error`, and why it is bounded
///
/// One event per **refused commit**, never per record and never on success: a
/// commit that succeeds writes nothing here, because that is the once-per-append
/// path `store.append committed` already carries.
fn note_commit_refused(header: &Header, refusal: FormatError) {
    let why = refusal.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("store.header", "commit refused")
            .with(
                "format_version",
                telemetry::Value::Uint(u64::from(header.format_version)),
            )
            .with(
                "record_stride",
                telemetry::Value::Uint(u64::from(header.record_stride)),
            )
            .with("generation", telemetry::Value::Uint(header.generation))
            .with("n_valid", telemetry::Value::Uint(header.n_valid))
            .with(
                "symbol_id",
                telemetry::Value::Uint(u64::from(header.symbol_id)),
            )
            .with("why", telemetry::Value::Str(&why)),
    );
}

/// The best-positioned decoded slot whose generation is below `below`.
///
/// `below` is exclusive and `None` means no bound, so the caller can ask again
/// for the next-newest commit after the newest one failed its check. Because
/// slot position is `generation % slot_count`, two positionally-valid slots
/// can never share a generation, and the bound therefore strictly shrinks the
/// candidate set on every call.
///
/// The error it reports when there is no candidate is the most *specific*
/// refusal any slot produced. A slot that is merely blank fails the magic
/// check, which says nothing; a slot naming a retired version, a wrong stride
/// or a failed checksum says everything, and reporting the blank one instead
/// would diagnose an intact file from another version as a destroyed header.
///
/// # A misplaced slot refuses the whole read
///
/// [`FormatError::SlotPositionMismatch`] is documented as "refused rather than
/// tolerated", and until this was fixed it was tolerated: the fault was
/// consulted only when NO slot decoded, so a commit written to the wrong slot
/// was swallowed whenever any other slot still read. The failure that hides
/// behind that is not hypothetical — slot position is
/// `generation % slot_count`, so a commit in the wrong slot has landed on the
/// slot holding the *previous* generation and destroyed it. Returning the
/// older generation with `Ok` reports a healthy file that has silently lost
/// every commit since.
///
/// So a position mismatch is now terminal for the read, whatever else decoded.
/// `CLAUDE.md` section 4 bans a fallback that hides a failure: degrade loudly
/// and name the reason, or refuse.
fn best_candidate(region: &[u8], below: Option<u64>) -> Result<(Header, Layout), FormatError> {
    let mut fault: Option<FormatError> = None;
    // Kept apart from `fault` because this one is not a last resort. It is
    // returned even when a candidate was found.
    let mut misplaced: Option<FormatError> = None;

    // `max_by_key` rather than a hand-written comparison: two positionally
    // valid slots can never share a generation, so `>` and `>=` would behave
    // identically there and no test could tell them apart.
    let best = (0u64..)
        .zip(region.chunks(SLOT_STRIDE_LEN).take(MAX_SLOTS))
        .filter_map(|(index, chunk)| match Header::decode_parts(chunk) {
            Err(refusal) => {
                // KEEP THE MOST INFORMATIVE, not the first one seen. Was
                // `fault.get_or_insert(refusal)`, which is "first in slot
                // order" wearing the words "most specific".
                if is_specific(refusal)
                    && fault.is_none_or(|held| informativeness(refusal) > informativeness(held))
                {
                    fault = Some(refusal);
                }
                None
            }
            Ok((header, layout)) => {
                let expected = header.generation % layout.slot_count();
                if index == expected {
                    Some((header, layout))
                } else {
                    misplaced.get_or_insert(FormatError::SlotPositionMismatch {
                        expected,
                        found: index,
                    });
                    None
                }
            }
        })
        .filter(|(header, _)| below.is_none_or(|limit| header.generation < limit))
        .max_by_key(|(header, _)| header.generation);

    // The mismatch wins over any candidate: a slot in the wrong place means a
    // writer wrote where it must not, and the generation this call would
    // otherwise return is exactly the one that write may have destroyed.
    match misplaced {
        Some(refusal) => Err(refusal),
        None => best.ok_or(fault.unwrap_or(FormatError::NoValidHeader)),
    }
}

/// Whether a refusal says something about the file, or only that a slot is not
/// a header at all.
const fn is_specific(refusal: FormatError) -> bool {
    !matches!(
        refusal,
        FormatError::NotABarFile | FormatError::SlotTooShort { .. }
    )
}

/// How informative a refusal is, so "most specific" can mean something.
///
/// [`best_candidate`]'s doc promises *"the most specific refusal any slot
/// produced"*, and the mechanism behind that sentence was `get_or_insert` --
/// which keeps whichever specific refusal came first in SLOT ORDER. There was
/// no ranking at all, so a file whose slot 0 was checksum-damaged and whose
/// slot 1 held an intact version-1 header reported "the header is unreadable"
/// instead of "this is a version 1 file": precisely the false diagnosis the
/// paragraph says the mechanism exists to prevent.
///
/// Two ranks are enough, and more would be invented precision:
///
/// - **2 — it identifies the FILE.** [`FormatError::RetiredVersion`] and
///   [`FormatError::UnknownVersion`] say what this file *is*. An operator can
///   act on that; it is not a damage report.
/// - **1 — it identifies damage to a SLOT.** A failed checksum, a wrong
///   stride, a counter past the end. True, and less useful, because another
///   slot may explain the file.
const fn informativeness(refusal: FormatError) -> u8 {
    match refusal {
        FormatError::RetiredVersion(_) | FormatError::UnknownVersion(_) => 2,
        _ => 1,
    }
}

/// How many whole slot positions the region holds, capped at the family bound.
///
/// Counted by folding rather than `count()` so the answer is a `u64` without a
/// `usize` cast this workspace would deny, and bounded by
/// [`crate::format::MAX_SLOTS`] so the fold cannot overflow.
fn whole_slots(region: &[u8]) -> u64 {
    region
        .chunks_exact(SLOT_STRIDE_LEN)
        .take(MAX_SLOTS)
        .fold(0u64, |seen, _| seen + 1)
}

/// Every byte of a slot except the four the checksum itself occupies.
///
/// Covering the reserved tail as well as the fields means a flipped bit
/// *anywhere* in the 64 bytes is detected — there is no window a corruption
/// can land in and be called clean. All 512 of them are walked by
/// `store::fault::a_single_bit_flip_in_any_header_byte_is_detected`.
///
/// **The domain is bytes `0..OFF_CRC` then `OFF_RESERVED..SLOT_LEN` — 60 bytes
/// with a four-byte hole — and it is frozen.** Zero-filling the hole to get one
/// contiguous 64-byte buffer is the tempting simplification and it is a
/// *different number*: every slot already on disk would fail
/// [`FormatError::SlotChecksum`], and by [`Header::read_region`] a failed slot
/// checksum condemns the whole file. Because it is a change of domain rather
/// than of algorithm, no round-trip test and no published check value would
/// notice. `store::unit::the_covered_domain_is_the_slot_minus_its_checksum`
/// pins it against a hardcoded constant.
///
/// Returns the two runs as owned arrays rather than as sub-slices because this
/// workspace denies slice indexing, and 60 bytes copied once per header commit
/// or decode is not on any path that repeats: the per-block checksum is the hot
/// one, and it takes its bytes directly.
fn covered(slot: &[u8; SLOT_LEN]) -> ([u8; OFF_CRC], [u8; SLOT_LEN - OFF_RESERVED]) {
    let mut head = [0u8; OFF_CRC];
    for (dst, src) in head.iter_mut().zip(slot.iter()) {
        *dst = *src;
    }
    let mut tail = [0u8; SLOT_LEN - OFF_RESERVED];
    for (dst, src) in tail.iter_mut().zip(slot.iter().skip(OFF_RESERVED)) {
        *dst = *src;
    }
    (head, tail)
}

/// Reads `N` little-endian bytes at `offset`.
///
/// Index-free by construction — this workspace denies slice indexing, and a
/// header decoder is exactly the place a panicking index would eventually be
/// reached by a corrupt file. Callers check `slot.len() >= SLOT_LEN` first, so
/// no read is ever short.
fn le_bytes<const N: usize>(slot: &[u8], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    for (dst, src) in out.iter_mut().zip(slot.iter().skip(offset)) {
        *dst = *src;
    }
    out
}
