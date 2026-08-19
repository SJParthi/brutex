//! The record, the shared constants, and every error the bytes can produce.
//!
//! ```text
//! byte 0                 32768                                      EOF
//!   ├──── header region ────┼─── record 0 ─┼─── record 1 ─┼── … ───────┤
//!    2 slots × 16384 spacing    56 bytes       56 bytes
//! ```
//!
//! **Address of record *i*: `header_len + i·stride`.** An add, a multiply, a
//! load. No search, no decode, no allocation — which is the entire reason the
//! format exists and the only thing that makes bar lookup O(1).
//!
//! The two numbers in that formula are **not** read from this module by the
//! read path. They come from [`crate::layout::Layout`], selected by the file's
//! own `format_version`. The constants here are version 2's definition, and
//! `store::unit::the_constants_are_the_current_versions_layout` pins them to
//! it.
//!
//! # Why this is version 2 and not an edited version 1
//!
//! `CLAUDE.md` §3 rule 8: *store format versions are never mutated in place*.
//! Version 1 — the geometry `docs/02-store-format.md` described before this
//! change — had a **64-byte header region holding one slot**, an 8-byte
//! `header_crc` at offset 48 covering bytes 0..48, and a byte-addressed
//! 4096-byte checksum block. Every one of those numbers is different now: the
//! header region is two slots spaced a failure unit apart, the checksum is
//! four bytes at offset 56 covering everything except itself, a `generation`
//! field was inserted at offset 16, and the block is counted in records.
//!
//! Keeping `MAGIC = b"BRUTEXB1"` and `FORMAT_VERSION = 1` across that would
//! make two incompatible geometries answer to one number, which is precisely
//! what [`crate::layout`]'s dispatch exists to make impossible. So the change
//! mints a version: [`MAGIC`] is `b"BRUTEXB2"` and [`FORMAT_VERSION`] is 2.
//!
//! Version 1 is **retired**, not deleted — see [`RETIRED_VERSIONS`]. It is
//! refused by number, naming the reason, so a version-1 file is never read at
//! version 2's offsets and never reported as a destroyed header.

use brutex_core::price::Paisa;

/// Bytes of fields in one header slot. A **family** constant: every format
/// version fills 64 bytes at the start of its slot.
///
/// This is what makes a header self-describing without knowing its version
/// first. A reader that cannot yet know the layout still knows where each slot
/// begins and how much of it to checksum, so it can decode whichever slots
/// survived a crash and let the magic and version inside them decide the rest.
/// Freezing the slot size is the price of that bootstrap, and it is cheap: a
/// new field takes reserved space in a new version, never more slot.
pub const SLOT_LEN: usize = 64;

/// Bytes from the start of one header slot to the start of the next. A
/// **family** constant, for the same bootstrap reason as [`SLOT_LEN`].
///
/// # Why 16384 and not 64
///
/// The two-slot header is only worth having if a write to one slot cannot
/// damage the other. Storage does not fail at byte granularity: the smallest
/// unit a device programs or a filesystem writes back is a block or a page,
/// and a partially programmed unit takes **everything in it** with it. At
/// 64-byte spacing both slots share one such unit, so the redundancy is
/// nominal — which is exactly why ext4 and F2FS separate their redundant
/// superblocks and checkpoint packs by whole blocks rather than by bytes.
///
/// Measured on the two hosts this repository runs on:
///
/// | Host | Device block | Page |
/// |---|---|---|
/// | Apple M4 Pro, macOS, APFS (`diskutil info /`, `sysctl hw.pagesize`) | 4096 | 16384 |
/// | GitHub CI runner, `x86_64` Linux, ext4 | 4096 | 4096 |
///
/// 16384 is the largest of those, so one slot per 16384 bytes puts the two
/// slots in different failure units on **both** hosts. It is a constant rather
/// than a query of the host, because a geometry that differed between the two
/// would make the same bytes verify differently on each — `CLAUDE.md` §3
/// rule 5, the same argument that rejected a page-sized [`BLOCK_LEN`].
///
/// *Rejected — 4096.* It equals the device block on both hosts but is smaller
/// than this machine's 16384-byte page, and page-granular writeback is a real
/// failure unit. The saving is 16 KiB per file, against a month file of
/// roughly 440 KiB.
///
/// **Honest limit:** this separates the slots by every unit that was measured.
/// It is not a proof. A failure whose granularity exceeds 16384 bytes — an
/// erase block, a dead device — takes both slots, and no arrangement inside
/// one file survives that. See the limits this crate reports.
pub const SLOT_STRIDE: u64 = 16_384;

/// The most header slots any version may declare. A **family** bound.
///
/// [`crate::header::Header::read_region`] must know how much of a buffer can
/// possibly be header before it knows the version, or a long buffer lets
/// record bytes audition as header slots. This is that bound.
pub const MAX_SLOT_COUNT: u64 = 2;

/// [`MAX_SLOT_COUNT`] as a `usize`, for iterator bounds.
///
/// Written as a literal in both widths rather than converted: this workspace
/// denies casts that can truncate, and a fallible conversion here would have a
/// failure arm no test could ever reach. The assertion below keeps the two
/// honest.
pub const MAX_SLOTS: usize = 2;

const _: () = assert!(MAX_SLOT_COUNT == 2 && MAX_SLOTS == 2);

/// The seven bytes shared by every version's magic.
///
/// Checked before the version is, so a file that is not a bar file at all is
/// named as such instead of being reported as an absurd version number.
pub const MAGIC_FAMILY: [u8; 7] = *b"BRUTEXB";

/// Identifies a version-2 bar file.
pub const MAGIC: [u8; 8] = *b"BRUTEXB2";

/// The only format version this build writes.
pub const FORMAT_VERSION: u16 = 2;

/// Versions that existed and are no longer readable by this build.
///
/// Version 1 described a 64-byte single-slot header with an 8-byte checksum at
/// offset 48 and a byte-addressed 4096-byte block. That header shape is not
/// this one, so decoding a version-1 file with this decoder would lift every
/// field from the wrong offset.
///
/// It is listed rather than forgotten so the refusal can say *why*. A version
/// that is merely absent from [`crate::layout::Layout::KNOWN`] reports
/// [`FormatError::UnknownVersion`], which reads as "this file is from the
/// future"; a retired version reports [`FormatError::RetiredVersion`], which
/// is the truth. `CLAUDE.md` §3 rule 8 makes the number append-only: it is
/// never reused for a different geometry.
///
/// No version-1 file can exist — version 1 never had a reader or a writer in
/// this repository, only constants. The entry costs one comparison and removes
/// the only way this build could misread one if that assumption is wrong.
///
/// A fixed-size array rather than a slice so the retirement test can be a
/// `const fn` that destructures it: retiring a second version changes the
/// length, which is a compile error at every destructuring site rather than a
/// silent drift.
pub const RETIRED_VERSIONS: [u16; 1] = [1];

/// Version 2: bytes of header region before the first record.
///
/// [`MAX_SLOT_COUNT`] slots at [`SLOT_STRIDE`] spacing. One slot could not be
/// updated atomically — see [`crate::header`].
///
/// Spelled as a literal rather than the product because that product needs a
/// `usize`→`u64` cast, and this workspace denies casts that can truncate.
/// `store::unit::the_constants_are_the_current_versions_layout` proves the
/// relationship instead, with a checked conversion.
pub const HEADER_LEN: u64 = 32_768;

/// Version 2: header slots. Writes alternate between them.
pub const SLOT_COUNT: u64 = 2;

/// Version 2: bytes per record. Read it from the layout; never assume it.
pub const RECORD_STRIDE: u64 = 56;

/// Version 2: whole records covered by one checksum block.
pub const RECORDS_PER_BLOCK: u64 = 73;

/// Version 2: bytes per checksummed block.
///
/// A whole multiple of [`RECORD_STRIDE`], which is the entire point: a record
/// cannot begin in one block and end in the next, so verifying it needs one
/// checksum and never two. See [`crate::layout`].
pub const BLOCK_LEN: u64 = RECORD_STRIDE * RECORDS_PER_BLOCK;

const _: () = assert!(BLOCK_LEN.is_multiple_of(RECORD_STRIDE));
const _: () = assert!(BLOCK_LEN / RECORD_STRIDE == RECORDS_PER_BLOCK);
const _: () = assert!(RECORDS_PER_BLOCK > 0);
const _: () = assert!(HEADER_LEN == SLOT_COUNT * SLOT_STRIDE);
const _: () = assert!(SLOT_COUNT <= MAX_SLOT_COUNT);
const _: () = assert!(BLOCK_LEN == 4088);
const _: () = assert!(SLOT_STRIDE >= 16_384);

// 56 x 73 = 4088, stated in both directions. `BLOCK_LEN == 4088` above pins
// the product; these pin the two factors, so neither can move while the other
// compensates and leaves the product — and every offset derived from it —
// looking untouched.
const _: () = assert!(RECORDS_PER_BLOCK == 73);
const _: () = assert!(BLOCK_LEN == 56 * 73);

// **A record can never straddle a block**, as a compile error rather than a
// walk. The last record of a block starts at `(RECORDS_PER_BLOCK - 1) *
// RECORD_STRIDE` inside it and ends exactly on the block's last byte; with the
// divisibility asserted above, every earlier record therefore ends strictly
// inside. `store::geometry::no_record_straddles_a_block` walks 5,000 indices
// for the same property at runtime — this is its closed form, and it holds
// before a test is ever run.
const _: () = assert!((RECORDS_PER_BLOCK - 1) * RECORD_STRIDE + RECORD_STRIDE == BLOCK_LEN);

/// Bit 0 of [`Header::flags`]: per-block checksums are present.
///
/// A file that sets it carries a sidecar of one [`crate::block`] checksum per
/// block; a file that does not carries none, and a reader must say so rather
/// than reporting an unverified block as verified.
///
/// [`Header::flags`]: crate::header::Header::flags
pub const FLAG_CHECKSUMS: u32 = 1;

/// Open interest is absent, as opposed to genuinely zero.
///
/// Spot indices carry no open interest and store this sentinel. Conflating it
/// with a real `0` would make a derivative series and an index series
/// indistinguishable on a field that decides which is which.
pub const OI_NULL: i64 = i64::MIN;

/// One bar. Seven `i64`, 56 bytes, no padding.
///
/// `#[repr(C)]` fixes the field order so the layout is the format rather than
/// a compiler choice. The assertions below are compile errors, not tests —
/// `docs/04-invariants.md` S-01.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bar {
    /// Microseconds since the Unix epoch, UTC. The **open** of the bar's
    /// interval — verified against 1,706,290 lake bars, not assumed.
    pub ts_micros: i64,
    /// Open, in paisa.
    pub open: i64,
    /// High, in paisa.
    pub high: i64,
    /// Low, in paisa.
    pub low: i64,
    /// Close, in paisa.
    pub close: i64,
    /// Contracts or shares. `0` is a real zero, not an absence.
    pub volume: i64,
    /// Open interest, or [`OI_NULL`] when absent.
    pub open_interest: i64,
}

/// Identifies an overlay file — the sidecar carrying fields a [`Bar`] has no
/// column for.
///
/// # Why a sibling file and not a wider `Bar`
///
/// Dhan's expired-options endpoint answers with the underlying's **spot** and
/// the contract's **implied volatility** beside the OHLC. Neither has a column
/// here, and both are wanted unchanged — `docs/00-charter.md` forbids deriving
/// what a vendor states.
///
/// Widening `Bar` was the obvious move and is the wrong one. `Bar` is the row
/// for EVERY instrument: an index, an equity, a future. Two option-only columns
/// would cost sixteen bytes on every spot bar ever written, and — because
/// `CLAUDE.md` §3 rule 8 forbids mutating a version in place — would mint a
/// format version 3 that RETIRES version 2, exactly as 2 retired 1. Every bar
/// already on disk would stop being readable to gain two fields most of them
/// have no value for.
///
/// `CLAUDE.md` §4 already names the alternative in the row that bans a dynamic
/// schema: *"a new field is a new file version at its own stride"*. A new
/// FILE, not a wider record. [`crate::path::FileKind::Overlay`] reserved the
/// name and the `.ovl` extension for it; this is the geometry.
/// # Why it stays inside [`MAGIC_FAMILY`] and takes a version bars will not
///
/// The first attempt put it outside the family as `BRUTEXO1`, and that was
/// wrong in a way only the next step revealed: [`crate::header::Header::validate`]
/// and [`crate::block::seal`] both take a [`crate::layout::Layout`], so a file
/// with no `Layout` has to duplicate the header, the commit counter, the CRC
/// and the block arithmetic. A second copy of that is a second place for a torn
/// write to be handled differently.
///
/// `Layout::declared` refuses a magic outside the family and refuses a RETIRED
/// version, and both refusals are right. The resolution is not to relax either
/// one: it is to take a version number the bar format will never reach. The
/// family byte identifies a GEOMETRY, and `9` is the sidecar's — 24-byte
/// stride against the bar's 56, distinguishable in the first eight bytes as
/// `BRUTEXB9` against `BRUTEXB2`.
///
/// It is still absent from [`crate::layout::Layout::KNOWN`], which answers
/// "which versions of a BAR file can this build read". A reader walking that
/// list is never offered the overlay, so the two cannot resolve against each
/// other even though they share a number space.
pub const OVERLAY_MAGIC: [u8; 8] = *b"BRUTEXB9";

/// The only overlay version this build writes.
pub const OVERLAY_VERSION: u16 = 9;

/// Bytes of one overlay record. Three `i64`, eight-aligned.
pub const OVERLAY_STRIDE: u64 = 24;

/// Overlay records in one block.
///
/// `24 × 170 = 4,080`, the largest whole multiple of the stride that fits the
/// same 4,096-byte failure unit the bar file uses. Chosen the same way
/// [`RECORDS_PER_BLOCK`] is and for the same reason: a torn write damages one
/// block, and a block that straddled the unit would damage two.
pub const OVERLAY_RECORDS_PER_BLOCK: u64 = 170;

const _: () = assert!(OVERLAY_STRIDE * OVERLAY_RECORDS_PER_BLOCK <= 4096);
const _: () = assert!(OVERLAY_STRIDE * (OVERLAY_RECORDS_PER_BLOCK + 1) > 4096);

/// One bar's worth of vendor-stated fields that [`Bar`] has no column for.
///
/// # One record per bar, keyed by the same stamp
///
/// [`Self::ts_micros`] is the bar's own open, so an overlay row and a bar row
/// are joined by value rather than by position. Position would be faster and
/// wrong the first time a bar is dropped by the session filter on one side and
/// not the other.
///
/// # Why both values are integers when one of them is a volatility
///
/// `CLAUDE.md` §7 keeps statistical values at full precision, and an implied
/// volatility is a statistical value — so `f64` is the type it deserves. It is
/// stored as millionths anyway, and the reason is `CLAUDE.md` §3 rule 5: a
/// rerun must be byte-for-byte identical. An `f64` has many bit patterns for
/// NaN and two for zero, so a sentinel written as a float is a sentinel that
/// can come back as different bytes. Millionths of a percent is finer than any
/// vendor states, and it makes the record exactly reproducible.
///
/// Both fields use [`OI_NULL`] as their absent marker, for the reason that
/// constant already gives: zero is a real reading and must not mean "missing".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Overlay {
    /// Microseconds since the Unix epoch, UTC — the open of the bar this
    /// overlays, and the only thing that joins the two.
    pub ts_micros: i64,
    /// The underlying's spot price at that stamp, in paisa, or [`OI_NULL`].
    pub spot: i64,
    /// Implied volatility in millionths, or [`OI_NULL`]. `125_000` is 0.125.
    pub iv_micros: i64,
}

const _: () = assert!(size_of::<Overlay>() == 24);
const _: () = assert!(align_of::<Overlay>() == 8);
const _: () = assert!(OVERLAY_STRIDE == 24);

impl Overlay {
    /// The spot as a value, or `None` when the vendor stated none.
    #[must_use]
    pub const fn spot(&self) -> Option<i64> {
        if self.spot == OI_NULL {
            None
        } else {
            Some(self.spot)
        }
    }

    /// The implied volatility as a value, or `None` when absent.
    #[must_use]
    pub const fn iv(&self) -> Option<i64> {
        if self.iv_micros == OI_NULL {
            None
        } else {
            Some(self.iv_micros)
        }
    }

    /// The bytes of one overlay record, little-endian.
    ///
    /// Field order and width mirror [`Bar::image`] exactly — three `i64` at
    /// 0, 8 and 16 — because the same block, checksum and commit machinery
    /// reads both, and a record that laid its fields out differently would
    /// need a second copy of that machinery to understand it.
    #[must_use]
    pub fn image(&self) -> [u8; OVERLAY_LEN] {
        let mut out = [0u8; OVERLAY_LEN];
        write_at(&mut out, 0, self.ts_micros.to_le_bytes());
        write_at(&mut out, 8, self.spot.to_le_bytes());
        write_at(&mut out, 16, self.iv_micros.to_le_bytes());
        out
    }

    /// Decodes one overlay record.
    ///
    /// Nothing is validated, for the reason [`Bar::decode`] gives: a record has
    /// no structure a lost write violates, and an all-zero overlay is a legal
    /// reading of zero spot and zero volatility. Detecting a lost write is
    /// [`crate::block`]'s job.
    ///
    /// # Errors
    ///
    /// [`FormatError::RecordTooShort`] when fewer than [`OVERLAY_STRIDE`] bytes
    /// are offered. Refused rather than zero-filled: inventing the missing
    /// bytes would manufacture a reading nobody wrote, and the null sentinel
    /// exists precisely so "nothing was stated" has its own value.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        if bytes.len() < OVERLAY_LEN {
            return Err(FormatError::RecordTooShort { len: bytes.len() });
        }
        let mut image = [0u8; OVERLAY_LEN];
        for (dst, src) in image.iter_mut().zip(bytes.iter()) {
            *dst = *src;
        }
        Ok(Self {
            ts_micros: i64::from_le_bytes(le_bytes(&image, 0)),
            spot: i64::from_le_bytes(le_bytes(&image, 8)),
            iv_micros: i64::from_le_bytes(le_bytes(&image, 16)),
        })
    }

    /// Whether this record states anything at all.
    ///
    /// An overlay with neither value is not worth a record, and the writer
    /// declines it: a file of empty rows costs a block per 170 bars and answers
    /// nothing a missing file does not answer better.
    #[must_use]
    pub const fn states_something(&self) -> bool {
        self.spot != OI_NULL || self.iv_micros != OI_NULL
    }
}

const _: () = assert!(size_of::<Bar>() == 56);
const _: () = assert!(align_of::<Bar>() == 8);
const _: () = assert!(RECORD_STRIDE == 56);

impl Bar {
    /// Whether open interest is absent rather than zero.
    #[must_use]
    pub const fn oi_is_null(&self) -> bool {
        self.open_interest == OI_NULL
    }

    /// Open interest as a value, or `None` when absent.
    #[must_use]
    pub const fn oi(&self) -> Option<i64> {
        if self.oi_is_null() {
            None
        } else {
            Some(self.open_interest)
        }
    }

    /// Whether the OHLC values are internally consistent.
    ///
    /// Checked at the write boundary, never at read: a file that already holds
    /// an impossible bar is corrupt, and the CRC is what detects that. This
    /// exists so an impossible bar never reaches the disk in the first place.
    ///
    /// Note what it deliberately does **not** claim: an all-zero record
    /// satisfies it. Zeros are a legal flat bar with a real open interest of
    /// zero, so this can never distinguish a bar from a lost write. That is
    /// [`crate::block`]'s job, and it is why `FLAG_CHECKSUMS` is not
    /// decoration.
    #[must_use]
    pub const fn ohlc_is_sane(&self) -> bool {
        let hi_ok = self.high >= self.open && self.high >= self.low && self.high >= self.close;
        let lo_ok = self.low <= self.open && self.low <= self.close;
        // AND NONE OF THEM IS BELOW ZERO, which the ordering alone never said.
        //
        // The three comparisons above are about the four prices' RELATIONSHIP,
        // and `-100 / -100 / -100 / -100` satisfies every one of them. So a bar
        // whose prices were all negative passed this check, was appended, and
        // the month was recorded as good — the ordering was consistent and
        // nothing else looked.
        //
        // Nothing on the exchanges this store holds trades below zero, so a
        // negative here is a decoder that read the wrong column, a price scale
        // that is wrong by a sign, or a vendor sending something that is not a
        // price. `crate::http::one_price` refuses one at the vendor boundary
        // where the original value is still visible and can be named; this is
        // the same rule at the WRITE boundary, which is the one every path
        // crosses — the CSV archives reach `append` without passing through
        // that decoder at all.
        //
        // `low` alone would be enough while the ordering holds, and all four are
        // tested anyway: this predicate must not depend on another clause of
        // itself being true, or a future edit to the ordering silently widens
        // what a price may be.
        let signs_ok = self.open >= 0 && self.high >= 0 && self.low >= 0 && self.close >= 0;
        hi_ok && lo_ok && signs_ok
    }

    /// Whether the two COUNTS on this bar can have happened.
    ///
    /// Separate from [`Self::ohlc_is_sane`] because they are separate claims,
    /// and folding counts into a predicate named for the OHLC is how the sign
    /// check went missing on the prices in the first place — a name stops
    /// being read once it looks familiar.
    ///
    /// `volume` is a count of shares or contracts. `CLAUDE.md` §7 is explicit
    /// that **zero means zero**: there is no sentinel here, so every negative
    /// is a decoder that read the wrong column or a scale applied twice.
    ///
    /// `open_interest` has exactly one legal negative, [`OI_NULL`], which is
    /// `i64::MIN` and means ABSENT rather than zero. Every other negative is
    /// the same class of fault as a negative volume.
    ///
    /// **What this was worth.** Before it existed, `survey` asked only about
    /// the four prices, so a bar with `volume: -1` was appended, checksummed,
    /// counted and recorded as good — and §3 rule 8 makes the month
    /// unrewritable. That is the same silent write D-0143 closed on the price
    /// side, left open one field over.
    #[must_use]
    pub const fn counts_are_sane(&self) -> bool {
        let volume_ok = self.volume >= 0;
        let oi_ok = self.open_interest == OI_NULL || self.open_interest >= 0;
        volume_ok && oi_ok
    }

    /// The close, as a price.
    #[must_use]
    pub const fn close_price(&self) -> Paisa {
        Paisa::from_raw(self.close)
    }

    /// The 56-byte image of this record, little-endian, in field order.
    ///
    /// # Why an explicit encoder rather than a cast of the struct
    ///
    /// `#[repr(C)]` fixes the field *order*, not the byte order, so a struct
    /// reinterpreted as bytes is the host's endianness — and this file is meant
    /// to be the same bytes on every host, which is the argument
    /// `docs/02-store-format.md` already makes about the block length. Casting
    /// would also need `unsafe`, and every crate here carries
    /// `#![forbid(unsafe_code)]`. Little-endian matches the header, whose every
    /// field `crate::header` writes with `to_le_bytes`, so one file has one byte
    /// order rather than two.
    ///
    /// It lives here rather than in the crate that ingests, because
    /// `docs/02-store-format.md` §3 is the authority for these bytes and a
    /// second encoder in `crates/pull` would be a second definition of the
    /// format, free to drift. `CLAUDE.md` §5's arrow points one way: `pull`
    /// depends on `store`.
    ///
    /// # What holds that claim up
    ///
    /// The paragraph above was, until D-0039, a claim this crate did not have:
    /// nothing called this method, and `store::fault` re-implemented the
    /// encoder privately — the exact second definition the paragraph forbids.
    /// Replacing the whole body with zeros left all 79 tests green. Three
    /// tests now hold it up, and they are named here because a comment that
    /// names no test is the shape the defect took:
    ///
    /// - `store::unit::the_record_image_is_the_pinned_bytes_of_a_known_bar`
    ///   pins all 56 bytes of one record as a literal array. A body that
    ///   returns zeros — or any other bytes — fails it.
    /// - `store::unit::the_image_is_little_endian_and_each_field_owns_its_own_offset`
    ///   holds up the byte order this comment claims, and walks all 448
    ///   (field, byte) positions so no two fields can swap.
    /// - `store::unit::decoding_the_image_returns_the_record_byte_for_byte`
    ///   round-trips every boundary a record has, [`OI_NULL`] included.
    ///
    /// And `store::fault::bitflip_detected` builds its record bytes by calling
    /// this method, so the format has one definition in fact and not only in
    /// this paragraph.
    #[must_use]
    pub fn image(&self) -> [u8; RECORD_LEN] {
        let mut out = [0u8; RECORD_LEN];
        write_at(&mut out, 0, self.ts_micros.to_le_bytes());
        write_at(&mut out, 8, self.open.to_le_bytes());
        write_at(&mut out, 16, self.high.to_le_bytes());
        write_at(&mut out, 24, self.low.to_le_bytes());
        write_at(&mut out, 32, self.close.to_le_bytes());
        write_at(&mut out, 40, self.volume.to_le_bytes());
        write_at(&mut out, 48, self.open_interest.to_le_bytes());
        out
    }

    /// Decodes one record.
    ///
    /// There is nothing to validate here and that is deliberate:
    /// `docs/02-store-format.md` §3 — *"a record has no structure that a lost
    /// write violates"*. An all-zero record is a legal flat bar. Detecting a
    /// lost write is [`crate::block`]'s job, and range validation happens at the
    /// ingest boundary before a byte is written, never on the way back in.
    ///
    /// # Errors
    ///
    /// [`FormatError::RecordTooShort`] when fewer than [`RECORD_STRIDE`] bytes
    /// are offered. Refused rather than zero-filled: a short tail is
    /// `store::unit`'s ragged-file case, and inventing the missing bytes would
    /// manufacture a bar nobody wrote.
    pub fn decode(bytes: &[u8]) -> Result<Self, FormatError> {
        let mut image = [0u8; RECORD_LEN];
        if bytes.len() < RECORD_LEN {
            return Err(FormatError::RecordTooShort { len: bytes.len() });
        }
        for (dst, src) in image.iter_mut().zip(bytes.iter()) {
            *dst = *src;
        }
        Ok(Self {
            ts_micros: i64::from_le_bytes(le_bytes(&image, 0)),
            open: i64::from_le_bytes(le_bytes(&image, 8)),
            high: i64::from_le_bytes(le_bytes(&image, 16)),
            low: i64::from_le_bytes(le_bytes(&image, 24)),
            close: i64::from_le_bytes(le_bytes(&image, 32)),
            volume: i64::from_le_bytes(le_bytes(&image, 40)),
            open_interest: i64::from_le_bytes(le_bytes(&image, 48)),
        })
    }
}

/// What a record file's writer needs of a record, and nothing more.
///
/// # Why this exists rather than a second writer
///
/// `BarFile::append` is about a hundred and forty lines of header advance,
/// commit ordering, offset arithmetic, durable write and block seal — and
/// exactly ONE of them is record-specific: the line that turns a record into
/// bytes. A second writer for the overlay would copy the other hundred and
/// thirty-nine, and a copy of a durability path is a second place for a torn
/// write to be handled differently.
///
/// So the writer is generic over this, and the two implementations are the two
/// things that genuinely differ: how wide a record is, when it happened, and
/// what its bytes are.
pub trait Row: Copy + PartialEq {
    /// Bytes of one record. The file's stride must equal this.
    const LEN: usize;

    /// When this record happened, in microseconds since the epoch, UTC.
    ///
    /// The writer orders and de-duplicates on this and nothing else, which is
    /// what lets a bar and its overlay be joined by value rather than by
    /// position.
    fn stamp(&self) -> i64;

    /// Appends this record's bytes to `out`.
    ///
    /// Writing INTO a buffer rather than returning an array, because the array
    /// length would have to be `Self::LEN` and a const-generic return is not
    /// expressible here. The buffer is reserved once per batch by the caller.
    fn write_into(&self, out: &mut Vec<u8>);

    /// Decodes one record from bytes already read off the file.
    ///
    /// The writer needs this for duplicate rejection: an append whose first
    /// stamp is already committed is compared against what is THERE, and a
    /// batch matching byte for byte is accepted as a rerun rather than
    /// refused. `CLAUDE.md` §3 rule 5 — reruns are safe — is that comparison.
    ///
    /// # Errors
    ///
    /// Whatever the record's own decoder refuses; in practice a short tail.
    fn read_from(bytes: &[u8]) -> Result<Self, FormatError>;

    /// Whether this record is internally possible.
    ///
    /// Checked at the write boundary, never at read: a file that already holds
    /// an impossible record is corrupt and the CRC is what detects that. This
    /// exists so an impossible one never reaches the disk.
    ///
    /// A [`Bar`] has four prices that must bracket each other. An [`Overlay`]
    /// has no such relation — a spot and a volatility constrain nothing about
    /// one another, and an all-zero overlay is a legal reading — so it answers
    /// `true` because there is genuinely nothing to violate, not because the
    /// check was skipped.
    fn is_sane(&self) -> bool;

    /// The two counts, when one of them is impossible.
    ///
    /// `Some((volume, open_interest))` names the pair so the refusal can quote
    /// both — a negative volume reported as "impossible bar" sends an operator
    /// to the four prices, which is the wrong diagnosis, and `CLAUDE.md` §4
    /// requires the reason to be the real one.
    ///
    /// `None` when the counts are fine, and `None` for an [`Overlay`], which
    /// carries no counts to be wrong about.
    fn bad_counts(&self) -> Option<(i64, i64)>;
}

impl Row for Bar {
    const LEN: usize = RECORD_LEN;

    fn stamp(&self) -> i64 {
        self.ts_micros
    }

    fn write_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.image());
    }

    fn read_from(bytes: &[u8]) -> Result<Self, FormatError> {
        Self::decode(bytes)
    }

    fn is_sane(&self) -> bool {
        self.ohlc_is_sane()
    }

    fn bad_counts(&self) -> Option<(i64, i64)> {
        if self.counts_are_sane() {
            None
        } else {
            Some((self.volume, self.open_interest))
        }
    }
}

impl Row for Overlay {
    const LEN: usize = OVERLAY_LEN;

    fn stamp(&self) -> i64 {
        self.ts_micros
    }

    fn write_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.image());
    }

    fn read_from(bytes: &[u8]) -> Result<Self, FormatError> {
        Self::decode(bytes)
    }

    fn is_sane(&self) -> bool {
        true
    }

    fn bad_counts(&self) -> Option<(i64, i64)> {
        None
    }
}

/// [`OVERLAY_STRIDE`] as a length, for the overlay image.
///
/// Separate from the stride for the reason `RECORD_LEN` is: one is a file
/// geometry and one is an array bound, and a `usize` cast at every use site
/// would be a cast this crate's lint table denies.
#[allow(clippy::cast_possible_truncation)]
pub const OVERLAY_LEN: usize = OVERLAY_STRIDE as usize;

/// [`RECORD_STRIDE`] as a length, for the record image.
///
/// Written as a literal in both widths rather than converted, for the reason
/// `crate::header` gives about every other paired constant: a conversion here
/// would carry a failure arm no test could reach. The assertion keeps the two
/// honest.
pub const RECORD_LEN: usize = 56;

const _: () = assert!(RECORD_STRIDE == 56 && RECORD_LEN == 56);

/// Writes `src` at `offset` in a record image.
///
/// Index-free, because `clippy::indexing_slicing` is denied across this
/// workspace and an encoder is exactly where a panicking index would eventually
/// be reached by an offset that moved.
fn write_at<const N: usize>(out: &mut [u8], offset: usize, src: [u8; N]) {
    for (dst, byte) in out.iter_mut().skip(offset).zip(src) {
        *dst = byte;
    }
}

/// Reads `N` little-endian bytes at `offset` from a record image.
fn le_bytes<const N: usize>(image: &[u8], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    for (dst, src) in out.iter_mut().zip(image.iter().skip(offset)) {
        *dst = *src;
    }
    out
}

/// Something about the bytes is wrong.
///
/// Every variant names the value it saw. A refusal that does not say what it
/// refused sends the reader back to a hex dump, which is where a "just retry
/// it" habit comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FormatError {
    /// A byte offset would not fit in `u64`.
    ///
    /// Refused rather than wrapped: a wrapped offset addresses a *different,
    /// valid-looking* record.
    OffsetOverflow,
    /// A slot buffer is shorter than [`SLOT_LEN`].
    SlotTooShort {
        /// Bytes the caller supplied.
        len: usize,
    },
    /// A record buffer is shorter than [`RECORD_STRIDE`].
    ///
    /// Distinct from [`FormatError::SlotTooShort`] because a short *record* and
    /// a short *slot* are two different files being wrong in two different
    /// places, and one message for both sends a reader to the wrong offset.
    RecordTooShort {
        /// Bytes the caller supplied.
        len: usize,
    },
    /// The header region is shorter than its version's slot count demands.
    ///
    /// A short region silently returns whichever older commit happened to fit,
    /// which is a fallback that hides a failure. Refused instead.
    HeaderRegionTooShort {
        /// Whole slots the region actually holds.
        slots: u64,
        /// Whole slots the version declares.
        need: u64,
    },
    /// The first seven bytes are not [`MAGIC_FAMILY`].
    NotABarFile,
    /// The header names a version this build does not know.
    ///
    /// Refused rather than guessed: `docs/02-store-format.md` mints a new
    /// version for every layout change precisely so old files stay readable at
    /// their own stride, and guessing defeats that.
    UnknownVersion(u16),
    /// The header names a version this build knows of and cannot decode.
    ///
    /// Distinct from [`FormatError::UnknownVersion`] because the two send an
    /// operator to opposite places: an unknown version means the file is newer
    /// than the build, a retired one means it is older. See
    /// [`RETIRED_VERSIONS`].
    RetiredVersion(u16),
    /// The magic's version byte disagrees with the `format_version` field.
    ///
    /// Two independent statements of the same fact; when they differ the file
    /// is not what either of them claims.
    MagicVersionMismatch(u16),
    /// The header names a stride other than the one its version defines.
    StrideMismatch(u16),
    /// The file is shorter than its version's header region.
    TooShortForHeader,
    /// `n_valid` claims more records than the file can hold.
    ///
    /// Trusting the counter over the file length would read past the end and
    /// return whatever bytes happened to be there.
    CounterExceedsFile,
    /// Appending would push `n_valid` past `u64::MAX`.
    CounterOverflow,
    /// The generation counter cannot be advanced again.
    ///
    /// Refused rather than wrapped, because a wrapped generation would make
    /// the *oldest* slot win and silently un-commit records.
    GenerationExhausted,
    /// An append's timestamps do not follow the records already committed.
    ///
    /// Bars are append-only in time. A batch that begins at or before the
    /// file's last timestamp is a re-pull landing on top of newer data, which
    /// no checksum can detect afterwards because the bytes are well formed.
    TimestampsOutOfOrder {
        /// The last timestamp already committed, or the batch's own first.
        previous: i64,
        /// The timestamp that did not follow it.
        next: i64,
    },
    /// A slot's stored checksum does not match its bytes.
    SlotChecksum {
        /// The checksum the slot carries.
        stored: u32,
        /// The checksum its bytes actually produce.
        computed: u32,
    },
    /// A block's stored checksum does not match its bytes.
    BlockChecksum {
        /// Which block.
        block: u64,
        /// The checksum the sidecar carries.
        stored: u32,
        /// The checksum the block's committed bytes actually produce.
        computed: u32,
    },
    /// A block index past the last block the commit counter covers.
    ///
    /// Verifying it would checksum bytes nobody committed and report the
    /// result as if it meant something.
    BlockNotCommitted {
        /// The block asked for.
        block: u64,
        /// How many blocks the counter covers.
        blocks: u64,
    },
    /// A caller offered a block's bytes at a length the geometry does not give
    /// that block.
    BlockLengthMismatch {
        /// Which block.
        block: u64,
        /// Bytes the caller supplied.
        len: usize,
        /// Bytes the block covers under the current commit counter.
        need: u64,
    },
    /// The file declares no block checksums, so a block cannot be verified.
    ///
    /// Reported rather than passed: "verified" and "there was nothing to
    /// verify against" are not the same answer.
    ChecksumsAbsent,
    /// A slot's generation says it should live in a different slot.
    ///
    /// A writer that puts a commit in the wrong slot can overwrite the only
    /// surviving copy of the previous one, so this is refused rather than
    /// tolerated.
    SlotPositionMismatch {
        /// Where `generation % slot_count` says it belongs.
        expected: u64,
        /// Where it was found.
        found: u64,
    },
    /// A declared layout is not a geometry a file could have.
    ///
    /// Refused at declaration rather than guarded at every use: a zero
    /// `records_per_block` is a division fault, and a header region that is
    /// not a whole number of slots puts a slot at an offset no reader looks
    /// at.
    DegenerateLayout {
        /// Which field of the declaration is impossible.
        field: &'static str,
    },
    /// No slot in the header region decoded.
    ///
    /// Not a torn tail — a torn tail is unobservable. This means every copy of
    /// the header is damaged, and there is nothing to fall back to that would
    /// not be a guess.
    NoValidHeader,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::OffsetOverflow => f.write_str("record offset overflows u64"),
            Self::SlotTooShort { len } => {
                write!(f, "header slot is {len} bytes, needs {SLOT_LEN}")
            }
            Self::RecordTooShort { len } => {
                write!(f, "record is {len} bytes, needs {RECORD_STRIDE}")
            }
            Self::HeaderRegionTooShort { slots, need } => {
                write!(f, "header region holds {slots} whole slots, needs {need}")
            }
            Self::NotABarFile => f.write_str("magic does not begin BRUTEXB"),
            Self::UnknownVersion(v) => write!(f, "unknown format version {v}"),
            Self::RetiredVersion(v) => {
                write!(f, "format version {v} is retired and cannot be decoded")
            }
            Self::MagicVersionMismatch(v) => {
                write!(f, "magic does not belong to format version {v}")
            }
            Self::StrideMismatch(s) => write!(f, "record stride {s} is not the version's stride"),
            Self::TooShortForHeader => f.write_str("file is shorter than its header region"),
            Self::CounterExceedsFile => {
                f.write_str("n_valid claims more records than the file holds")
            }
            Self::CounterOverflow => f.write_str("n_valid would overflow u64"),
            Self::GenerationExhausted => f.write_str("header generation cannot advance past u64"),
            Self::TimestampsOutOfOrder { previous, next } => {
                write!(f, "timestamp {next} does not follow {previous}")
            }
            Self::SlotChecksum { stored, computed } => {
                write!(f, "header slot checksum {stored:#010x} != {computed:#010x}")
            }
            Self::BlockChecksum {
                block,
                stored,
                computed,
            } => write!(
                f,
                "block {block} checksum {stored:#010x} != {computed:#010x}"
            ),
            Self::BlockNotCommitted { block, blocks } => {
                write!(f, "block {block} is past the {blocks} blocks committed")
            }
            Self::BlockLengthMismatch { block, len, need } => {
                write!(f, "block {block} was offered {len} bytes, covers {need}")
            }
            Self::ChecksumsAbsent => f.write_str("the file declares no block checksums"),
            Self::SlotPositionMismatch { expected, found } => {
                write!(
                    f,
                    "header slot {found} holds a commit belonging in {expected}"
                )
            }
            Self::DegenerateLayout { field } => {
                write!(f, "layout field {field} is not a geometry a file can have")
            }
            Self::NoValidHeader => f.write_str("no header slot survived; the header is unreadable"),
        }
    }
}

impl std::error::Error for FormatError {}
