//! The per-vendor counter file — layer 13 of `docs/07-o1-architecture.md`.
//!
//! # The question this exists to answer in one read
//!
//! *How much do I have?* Against the store's directory tree that is a walk of
//! roughly 248,000 files — one `stat` each — and it gets slower with every
//! ingest, which is precisely the decay `CLAUDE.md` exists to prevent. Law 3:
//! **never scan to answer a question; maintain a counter instead.**
//!
//! So each vendor has one file recording, per `(instrument, timeframe, month)`,
//! the row count and the first and last timestamp, and its header carries the
//! three totals — entries written, distinct keys, rows held — maintained on
//! every write. [`Manifest::total_rows`], [`Manifest::keys`] and
//! [`Manifest::entries`] are field reads.
//!
//! What that does **not** buy, said plainly: a *filtered* census — "how many
//! expired option series" — is still a walk of the manifest's own entries. It
//! is a sequential read of one file rather than 248,000 directory operations,
//! which is the difference the layer is about, but it is not O(1) and is not
//! claimed to be. A per-segment counter in the header would make it one, and it
//! would be a **new field at a new format version**, never a dynamic schema —
//! `CLAUDE.md` §4. See `docs/06-limits.md` §17.
//!
//! # Bytes on disk
//!
//! `docs/02-store-format.md` §11 is the authority; this is the summary.
//!
//! ```text
//! byte 0                 32768                                       EOF
//!   ├──── header region ────┼─── entry 0 ──┼─── entry 1 ──┼── … ──────┤
//!    2 slots × 16384 spacing    64 or 128      64 or 128
//! ```
//!
//! **Address of entry *i*: `HEADER_LEN + i·stride`,** where the stride comes
//! from the file's own [`Layout`] and is **never a constant on the read path**.
//! [`Layout::offset_of`] is an add and a multiply — law 4, arithmetic beats
//! lookup.
//!
//! Everything here is built out of one 64-byte unit with a CRC-32C over its
//! first 60 bytes: the header slot is one, a **version-1 entry** is one, and a
//! **version-2 entry is two** — a version-1 entry byte for byte, then the
//! month's first and last close. So one checksum helper serves all of them,
//! nothing straddles a cache line, and every byte of an entry is covered by
//! exactly one checksum.
//!
//! Version 1 is read and never written. Version 2 is what this build writes,
//! and D-0067 says why the closes are in the census at all: `/store.json` must
//! serve a month's percentage change, and deriving it from the bars costs
//! ~17.5 GB to extract 492 KB.
//!
//! **[`CLOSE_NULL`] is the not-recorded sentinel and zero means zero.** A month
//! that really closed at zero paisa records a zero; a month nobody has priced
//! records `i64::MIN`; a month the census does not hold answers `None`. Three
//! facts, three answers.
//!
//! # What is reused from `crates/store`, and what is not
//!
//! Reused, as **code**:
//!
//! * [`store::crc::crc32c`] — the same polynomial, the same table, verified
//!   against the same published check value. Re-deriving it here would be a
//!   second implementation of a checksum, which is the one kind of duplication
//!   that fails silently: two kernels that disagree produce two files that each
//!   verify only against themselves.
//! * [`store::path::Timeframe`] and [`store::path::YearMonth`] — the manifest's
//!   key must be the tuple that names a bar file, and a second definition of
//!   "which timeframes exist" would let the manifest record a month the store
//!   cannot address.
//! * [`store::format::SLOT_LEN`], [`store::format::SLOT_STRIDE`] and
//!   [`store::format::MAX_SLOT_COUNT`] — the header-slot *geometry*, not the
//!   header. The 16,384-byte spacing is the measured failure-unit argument in
//!   `store::format::SLOT_STRIDE`, and it is the same argument here: two slots
//!   64 bytes apart share one device block and the redundancy is nominal.
//!
//! Reused as **reasoning**, deliberately not as code:
//!
//! * *The commit counter is the authority* (D-0004). A reader takes entries
//!   `0..n_valid`, so a torn entry at the tail is not merely unlikely, it is
//!   unobservable.
//! * *A header update is one write of one self-checked unit.* [`Commit`]
//!   carries one offset and one 64-byte buffer, so there is no API here that
//!   can update the counter and the checksum separately.
//! * *Alternating slots.* Commit *g* goes to slot `g % 2`, so consecutive
//!   commits never write the same slot and a crash mid-write leaves the other
//!   one whole.
//!
//! **Not reused: [`store::header::Header`] itself.** Its fields are a bar
//! file's — `symbol_id`, `timeframe_secs`, `first_ts_micros` — and this file
//! holds counters instead. Widening that struct to serve both would make one
//! `format_version` describe two geometries, which is exactly what
//! `store::layout`'s dispatch exists to make impossible. Its discontiguous
//! checksum domain is also not reused: that shape exists because a bar file's
//! slot has a reserved field *after* its checksum, and this one does not, so
//! the checksum sits last and covers one contiguous run.
//!
//! # A torn write is detectable, never silently believed
//!
//! Three independent mechanisms, each covering what the others cannot:
//!
//! | Failure | What catches it |
//! |---|---|
//! | a half-written entry at the tail | `n_valid` — the reader never looks at it |
//! | a half-written or corrupt entry *below* `n_valid` | that entry's own CRC-32C, and then the previous generation is tried |
//! | a half-written header slot | that slot's own CRC-32C, and the other slot |
//! | a header published before the entries it counts | [`ManifestHeader::validate`] against the region, then [`Manifest::load`] walking down the generations |
//! | a header whose counters do not match its entries | [`Manifest::load`] recomputes both and refuses a disagreement |
//!
//! The last one is what makes the counter worth having. A counter that is never
//! checked against the thing it counts is a number, not a measurement — and
//! this whole module exists so that number can be trusted without a walk. The
//! check runs once, on load, where a walk is already being paid.
//!
//! **Falling back is never silent.** Every generation stepped over on the way
//! to the one that loaded is reported by [`Manifest::degraded_reason`], because
//! `CLAUDE.md` §4 admits exactly two behaviours and "returned `Ok` on a file it
//! had just proved was damaged" is neither of them. Until D-0036 the fault was
//! computed, stored in a local, and dropped on the success path: a manifest
//! whose newest header slot failed its checksum loaded silently at the previous
//! generation, a committed month vanished from the census, and no API said so.
//!
//! # What the counters do NOT check: the bar files themselves
//!
//! The header is checked against **this file's own entries** and against
//! nothing else. `bars/` is never consulted, and no API here can tell that an
//! entry claiming 7,312 rows describes a month with no bar file at all, or that
//! a bar file written by a pull that crashed before its append is missing from
//! the census. The manifest is a record of what a writer *said* it wrote.
//!
//! That is inherent rather than an oversight: confirming it means walking
//! `bars/`, which is the ~248,000 directory operations this layer exists to
//! avoid, so a reconciliation pass is a deliberate, occasional, O(files)
//! operation and is not built. `docs/06-limits.md` §17 records it as an
//! open gap rather than leaving it implied.
//!
//! # Nothing here writes
//!
//! Like `crates/store`, this crate performs no I/O. [`Manifest::record`] returns
//! an [`Append`] — one entry image at one offset — and a [`Commit`] — one slot
//! image at one offset, with the byte offset through which the entry must
//! already be durable. A writable memory mapping stays banned: it raises
//! `SIGBUS` on a full disk and a signal cannot be caught.
//!
//! [`Manifest::image`] is the same statement at the scale of a whole file: it
//! returns the complete bytes — header region then entries, every checksum
//! computed — and [`Manifest::open_image`] reads them back. Those two are for
//! the file that does not exist yet and for one being replaced whole; the
//! incremental path is still `record`, which is 128 bytes however large the
//! census is. **A writer must produce bytes this module's own reader accepts**,
//! and the round trip that proves it goes through [`Manifest::load`] unchanged.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use brutex_core::error::InstrumentError;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use brutex_core::vendor::Vendor;
use store::crc::crc32c;
use store::format::{MAX_SLOT_COUNT, MAX_SLOTS, SLOT_LEN, SLOT_STRIDE};
use store::path::{MAX_VENDOR_LEN, PathError, Timeframe, YearMonth};

/// The directory manifests live in, beside `bars/`.
pub const MANIFEST_ROOT: &str = "manifest";

/// The extension one manifest carries.
pub const MANIFEST_EXTENSION: &str = ".man";

/// Identifies a version-1 manifest.
///
/// `BRUTEXM` rather than `BRUTEXB`: a manifest and a bar file must never be
/// mistaken for one another, and the family check is the first thing either
/// decoder does.
///
/// **This constant keeps meaning version 1 forever.** It is not "the magic this
/// build writes" — that is [`MAGIC_V2`] — because `CLAUDE.md` §3 rule 8 makes a
/// version an append-only identifier. A constant that followed the current
/// version would silently renumber an existing geometry, which is the same
/// argument `store::layout::FORMAT_VERSION_2` makes one crate away.
pub const MAGIC: [u8; 8] = *b"BRUTEXM1";

/// Identifies a version-2 manifest — the one this build writes.
pub const MAGIC_V2: [u8; 8] = *b"BRUTEXM2";

/// The seven bytes shared by every manifest version.
pub const MAGIC_FAMILY: [u8; 7] = *b"BRUTEXM";

/// The format version this build **writes**. Version 1 is read, never written.
///
/// `Layout::KNOWN` is what this build **reads**, and it holds both.
pub const FORMAT_VERSION: u16 = 2;

/// Bytes per header slot, and per checksummed image unit.
///
/// One 64-byte unit with a CRC-32C over its first 60 bytes serves the header
/// slot, a version-1 entry, and each half of a version-2 entry. That is why
/// there is one checksum helper here rather than three: two kernels that
/// disagree produce two files that each verify only against themselves.
pub const IMAGE_LEN: usize = 64;

/// Bytes per entry at [`FORMAT_VERSION`], in the width an offset is computed in.
///
/// **Version 1's stride is 64 and it is still read.** Nothing on the read path
/// may use this constant to address an entry; [`Layout::entry_stride`] is the
/// one that dispatches, and this is the value the version this build writes
/// declares. D-0067.
pub const ENTRY_STRIDE: u64 = 128;

/// Bytes per entry at [`FORMAT_VERSION`], in the width the header stores it.
pub const ENTRY_STRIDE_U16: u16 = 128;

/// Bytes per entry at [`FORMAT_VERSION`], in the width a slice is chunked at.
pub const ENTRY_LEN: usize = 128;

/// The widest entry stride a [`Layout`] row may declare.
///
/// Derived, not chosen: `HEADER_LEN + MAX_ENTRIES · stride` must fit a `u64`,
/// because [`Layout::offset_within_bounds`] is total past the ordinal check.
/// Ten orders of magnitude above anything a real format would use, and it is
/// here so that "the arithmetic cannot overflow" is a checked property of every
/// declared row rather than a sentence about the two rows that exist today.
pub const MAX_ENTRY_STRIDE: u64 = (u64::MAX - HEADER_LEN) / MAX_ENTRIES;

/// Version 1's entry stride, in the width an offset is computed in.
const V1_ENTRY_STRIDE: u64 = 64;

/// Version 1's entry stride, in the width a slice is chunked at.
const V1_ENTRY_STRIDE_LEN: usize = 64;

/// **The close that was never recorded.** Not zero, and never rendered as one.
///
/// A close is a price in paisa, so every real value is at or above zero and
/// `i64::MIN` is not a price any tick grid can produce. This is the same
/// convention `store::format::OI_NULL` states for open interest — *one* value
/// that cannot be a real quantity means "not recorded" — applied to a second
/// field, rather than a second mechanism that could drift out of step with the
/// first. `CLAUDE.md` §7: **zero means zero**, and a month whose bars really
/// closed at zero paisa records a zero and is told apart from a month whose
/// closes nobody has read.
///
/// The 31,493 keys already committed under version 1 have no closes at all, and
/// after the upgrade they carry this. An operator reading `/store.json` can
/// therefore tell "no data yet" from "the price really was that", which is the
/// whole reason the sentinel exists rather than a default.
pub const CLOSE_NULL: i64 = i64::MIN;

/// Header slots. Writes alternate between them.
pub const SLOT_COUNT: u64 = 2;

/// Bytes of header region before the first entry.
///
/// Spelled as a literal rather than the product, because that product needs a
/// `usize`→`u64` conversion and this workspace denies a cast that can truncate.
/// The assertion below is what keeps it honest.
pub const HEADER_LEN: u64 = 32_768;

/// The most entries one manifest may hold.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary.
/// Measured scale is ~248,000 files across every vendor and expiry;
/// 2,097,152 is 8.4× that and caps one manifest at 134 MB. The refusal names
/// the number, so an operator who legitimately outgrows it is told what to
/// raise rather than being handed an allocation the size of the file.
pub const MAX_ENTRIES: u64 = 2_097_152;

/// [`MAX_ENTRIES`] as a `usize`, for the map reservation.
///
/// Written as a literal in both widths rather than converted, for the same
/// reason [`HEADER_LEN`] is: there is no cast to deny and no fallible
/// conversion whose failure arm no test could ever reach. The assertion keeps
/// the two honest.
const MAX_ENTRIES_LEN: usize = 2_097_152;

/// How much room a loaded index reserves, as a multiple of the committed entry
/// count. **Two, and the derivation is the whole point of the constant.**
///
/// # What went wrong with one
///
/// Until D-0040 the index was reserved to *exactly* `n_valid` and the doc
/// comment beside it claimed "O(1) **worst case** rather than average". That
/// claim was true of the lookup path and false of the append path, and no bench
/// in the repository touched the append path at all — which is why nobody saw
/// it.
///
/// `HashMap::with_capacity(n)` rounds `n` up to `7·2^k`, so for most `n` the
/// rounding leaves slack and for `n = 7·2^k` **exactly** it leaves none. At
/// those counts the very next new key rebuilt the whole table. Measured on this
/// workspace's own harness, `crates/pull/benches/ratio.rs` C-13, appending 256
/// new months to a freshly loaded census, minimum of twelve reloads,
/// picoseconds per append:
///
/// | census | reserved before | ps/append before | reserved now | ps/append now |
/// |---|---|---|---|---|
/// | 1,792 = 7·2⁸ | 1,792 | 228,679 | 3,584 | 93,261 |
/// | 14,336 = 7·2¹¹ | 14,336 | 1,285,968 | 28,672 | 84,472 |
/// | 57,344 = 7·2¹³ | 57,344 | 5,100,585 | 114,688 | 95,214 |
///
/// The old column multiplies by 22 across a 32× census — and even its 1× base
/// was already rehashing, so the honest figure is the last row against the
/// flat cost: **5,100,585 ps against 95,214 ps, 53.6× removed**. The new column
/// is flat to 1.02×. `CLAUDE.md` §3 rule 4 names **result append** as one of
/// the operations that may not grow with the data.
///
/// The round counts 1,000 / 10,000 / 50,000 stayed under the ceiling in *both*
/// columns, because `with_capacity` happened to round them up to 1,792 / 14,336
/// / 57,344 and leave 792 / 4,336 / 7,344 spare slots. A bench that had only
/// visited round numbers would have reported this defect as absent, which is
/// why C-13 measures both sets.
///
/// # Why the factor is two, derived rather than chosen
///
/// `Manifest::walk` inserts `n_keys` distinct keys and
/// [`ManifestHeader::validate`] has already refused `n_keys > n_valid`, so the
/// index holds **at most `n_valid`** elements when the load returns. Reserving
/// `2·n_valid` therefore leaves **at least `n_valid` free slots**, and
/// [`Manifest::record`] adds at most one element per call. So the number of new
/// `(instrument, timeframe, month)` keys a session may append before any rehash
/// is possible is `n_valid` — a pull would have to double a vendor's entire
/// history in one run.
///
/// That bound is scale-free, which an additive headroom would not be: a fixed
/// "+1,000" is a number nothing in `docs/00-charter.md` supports and it gets
/// relatively weaker at every scale, and `CLAUDE.md` §3 rule 1 does not admit a
/// figure with no source.
///
/// # Past half the ceiling it stops being a headroom and becomes a proof
///
/// [`ManifestHeader::advance`] refuses a counter past [`MAX_ENTRIES`], so a
/// loaded manifest can accept at most `MAX_ENTRIES − n_valid` further appends
/// for the rest of its life. [`reservation_for`] caps the reservation at
/// `MAX_ENTRIES`, so once `n_valid >= MAX_ENTRIES/2` the reservation **is**
/// `MAX_ENTRIES` — more slots than `advance` will ever let a caller fill. Above
/// that census no append can rehash at all, unconditionally, and
/// `pull::unit::the_reservation_is_capped_at_the_design_ceiling` is where that
/// arithmetic is checked rather than believed.
///
/// # What it costs, measured
///
/// `size_of::<(EntryKey, Held)>()` is **152 bytes** on this workspace's target,
/// printed by the same bench and pinned by
/// `pull::unit::the_log_is_the_entry_region_in_order`. The *ceiling* is
/// unchanged in element count: a census at [`MAX_ENTRIES`] reserved
/// `MAX_ENTRIES` before this constant existed and reserves `MAX_ENTRIES` now —
/// about 637 MB of table either way. What doubles is the middle: a
/// 248,000-entry census (the measured scale in [`MAX_ENTRIES`]'s note) goes from
/// roughly 80 MB of table to roughly 159 MB.
///
/// **D-0067 widened the element from 136 bytes to 152**, by putting the month's
/// two closes in it. That is +11.8% on every figure in this note, and it is
/// stated rather than absorbed: the 574 MB and 144 MB this paragraph used to
/// name became the 637 MB and 159 MB above.
///
/// # What it does NOT buy
///
/// An unconditional O(1) worst-case append below half the ceiling. Appending
/// more than `n_valid` new keys to one loaded manifest rehashes once, at
/// `O(n_keys)`, and the only reservation that would remove that arm for every
/// census is [`MAX_ENTRIES`] on every load — 574 MB for a vendor holding three
/// months. `docs/06-limits.md` §23 records that plainly rather than leaving an
/// O(1) claim standing over an amortised bound. D-0040.
pub const APPEND_HEADROOM_FACTOR: usize = 2;

/// How many entries a loaded index reserves room for, given the committed entry
/// count its header published.
///
/// `min(n_valid · APPEND_HEADROOM_FACTOR, MAX_ENTRIES)` — see
/// [`APPEND_HEADROOM_FACTOR`] for why those are the two numbers. Public because
/// the alternative is a bound only an allocator can see: `CLAUDE.md` §3 rule 6
/// is not satisfied by an invariant whose only witness is a comment, and the
/// cap arm in particular is reachable only at a census of 1,048,576 entries,
/// which is 67 MB of entry bytes and not a thing a unit test should build.
///
/// The `unwrap_or` arm cannot be taken on any target this workspace builds for:
/// `n_valid` is at most [`MAX_ENTRIES`], which is 2²¹, and a `usize` narrower
/// than that could not address the region the entries were read from in the
/// first place. It sits in `Result::unwrap_or`, which is not this repository's
/// code to measure.
///
/// # Examples
///
/// ```
/// # use pull::manifest::{APPEND_HEADROOM_FACTOR, MAX_ENTRIES, reservation_for};
/// assert_eq!(reservation_for(0), 0);
/// assert_eq!(reservation_for(1_792), 3_584);
/// // The cap binds from half the design ceiling upward, and never above it.
/// assert_eq!(reservation_for(MAX_ENTRIES / 2), MAX_ENTRIES as usize);
/// assert_eq!(reservation_for(MAX_ENTRIES), MAX_ENTRIES as usize);
/// assert_eq!(APPEND_HEADROOM_FACTOR, 2);
/// ```
#[must_use]
pub fn reservation_for(n_valid: u64) -> usize {
    let census = usize::try_from(n_valid).unwrap_or(MAX_ENTRIES_LEN);
    census
        .saturating_mul(APPEND_HEADROOM_FACTOR)
        .min(MAX_ENTRIES_LEN)
}

/// [`store::format::SLOT_STRIDE`] as a `usize`, for slicing a region.
const SLOT_STRIDE_LEN: usize = 16_384;

/// [`HEADER_LEN`] as a `usize`, for building and for splitting a whole file.
///
/// A literal in both widths for the same reason [`HEADER_LEN`] is one: the
/// product needs a `usize`→`u64` conversion this workspace denies. The
/// assertion below is what keeps the two honest.
const HEADER_REGION_LEN: usize = 32_768;

const _: () = assert!(MAX_ENTRIES == 2_097_152 && MAX_ENTRIES_LEN == 2_097_152);
const _: () = assert!(SLOT_STRIDE == 16_384 && SLOT_STRIDE_LEN == 16_384);
const _: () = assert!(HEADER_LEN == 32_768 && HEADER_REGION_LEN == 32_768);
const _: () = assert!(HEADER_LEN == SLOT_COUNT * SLOT_STRIDE);
const _: () = assert!(SLOT_COUNT <= MAX_SLOT_COUNT);
const _: () = assert!(IMAGE_LEN == SLOT_LEN);
const _: () = assert!(ENTRY_STRIDE == 128 && ENTRY_STRIDE_U16 == 128 && ENTRY_LEN == 128);
const _: () = assert!(V1_ENTRY_STRIDE == 64 && V1_ENTRY_STRIDE_LEN == 64);
// A version-2 entry is exactly two of the 64-byte units this module already
// checksums, so its first half IS a version-1 entry image, byte for byte.
const _: () = assert!(ENTRY_LEN == 2 * IMAGE_LEN);
// 32,768 is a whole multiple of 128, so an entry is 64-byte aligned at both
// strides and never straddles a cache line.
const _: () = assert!(HEADER_LEN.is_multiple_of(ENTRY_STRIDE));
const _: () = assert!(MAGIC[0] == MAGIC_FAMILY[0] && MAGIC[7] == b'1');
const _: () = assert!(MAGIC_V2[0] == MAGIC_FAMILY[0] && MAGIC_V2[7] == b'2');
const _: () = assert!(CLOSE_NULL == i64::MIN);

/// Byte offset of the checksum, in both an entry and a header slot.
const OFF_CRC: usize = 60;

/// Byte offset of `magic` in a header slot.
const H_MAGIC: usize = 0;
/// Byte offset of `format_version` in a header slot.
const H_VERSION: usize = 8;
/// Byte offset of `entry_stride` in a header slot.
const H_STRIDE: usize = 10;
/// Byte offset of `generation` in a header slot.
const H_GENERATION: usize = 16;
/// Byte offset of `n_valid` in a header slot.
const H_N_VALID: usize = 24;
/// Byte offset of `n_keys` in a header slot.
const H_N_KEYS: usize = 32;
/// Byte offset of `total_rows` in a header slot.
const H_TOTAL_ROWS: usize = 40;
/// Byte offset of the vendor segment in a header slot.
const H_VENDOR: usize = 48;
/// Bytes the vendor segment occupies.
const H_VENDOR_LEN: usize = 8;

const _: () = assert!(MAX_VENDOR_LEN <= H_VENDOR_LEN);
const _: () = assert!(H_VENDOR + H_VENDOR_LEN <= OFF_CRC);

/// Byte offset of the symbol in an entry.
const E_SYMBOL: usize = 0;
/// Byte offset of `rows` in an entry.
const E_ROWS: usize = 24;
/// Byte offset of `first_ts_micros` in an entry.
const E_FIRST_TS: usize = 32;
/// Byte offset of `last_ts_micros` in an entry.
const E_LAST_TS: usize = 40;
/// Byte offset of `timeframe_secs` in an entry.
const E_TIMEFRAME: usize = 48;
/// Byte offset of `year` in an entry.
const E_YEAR: usize = 52;
/// Byte offset of `month` in an entry.
const E_MONTH: usize = 54;
/// Byte offset of the exchange code in an entry.
const E_EXCHANGE: usize = 55;
/// Byte offset of the segment code in an entry.
const E_SEGMENT: usize = 56;

const _: () = assert!(E_SYMBOL + SYMBOL_CAPACITY == E_ROWS);
const _: () = assert!(E_SEGMENT < OFF_CRC);
const _: () = assert!(OFF_CRC + 4 == IMAGE_LEN);

// Version 2's second half, offsets relative to the half rather than to the
// entry. It is its own 64-byte image with its own CRC-32C at the same offset
// 60, so `seal` and `verify` are reused as CODE rather than re-derived — the
// argument this module's header already makes about two checksum kernels.

/// Byte offset of the month's first close, within the closes half.
const C_FIRST_CLOSE: usize = 0;
/// Byte offset of the month's last close, within the closes half.
const C_LAST_CLOSE: usize = 8;

const _: () = assert!(C_FIRST_CLOSE + 8 == C_LAST_CLOSE);
// 16..60 is reserved and stays zero. `docs/02-store-format.md` §2: a future
// field takes reserved space in a NEW VERSION, never by reinterpreting this
// one.
const _: () = assert!(C_LAST_CLOSE + 8 < OFF_CRC);

/// The path of one vendor's manifest under `root`.
///
/// One allocation, bounded: the root, one directory, one file name whose
/// longest form is [`MAX_VENDOR_LEN`] plus the extension. The vendor segment is
/// a [`Vendor`], not a string, for the reason `store::path` gives — a `&str`
/// there could name any vendor, or none.
#[must_use]
pub fn manifest_path(root: &Path, vendor: Vendor) -> PathBuf {
    let mut out = PathBuf::with_capacity(
        root.as_os_str().len()
            + 2
            + MANIFEST_ROOT.len()
            + MAX_VENDOR_LEN
            + MANIFEST_EXTENSION.len(),
    );
    out.push(root);
    out.push(MANIFEST_ROOT);
    out.push(format!("{}{MANIFEST_EXTENSION}", vendor.as_str()));
    out
}

/// The geometry of one manifest format version, and the dispatch that selects
/// it.
///
/// # The stride is not a global
///
/// `docs/02-store-format.md` says "a format change mints a **new version** and
/// old files stay readable at their own stride". Until D-0067 that sentence was
/// prose here: [`ManifestHeader::decode`] compared `format_version` against one
/// constant and `entry_stride` against another, so this build could read exactly
/// one geometry and minting version 2 would have made every version-1 census on
/// disk unreadable. Reading one version's entries at the other's stride is
/// worse still — fields lifted from the wrong offsets are plausible integers,
/// silently wrong, which is `docs/00-charter.md` prohibition 6 exactly.
///
/// `crates/store` solved this for bar files and `store::layout` is the model
/// followed here, deliberately down to the shape of the table: every geometric
/// question is a method on a `Layout`, and the only way to obtain one is
/// [`Layout::for_version`], which refuses an unknown version **by number**.
///
/// # Adding version 3
///
/// Add a row to [`Layout::KNOWN`], built by [`Layout::declared`]. Resolution is
/// a search of that table for a matching `version`, so a new row cannot alter
/// what an existing row resolves to — proven by
/// `pull::unit::a_known_manifest_version_keeps_its_stride_after_a_new_one_exists`,
/// which runs the production resolver against a table that already holds a
/// third version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    version: u16,
    magic: [u8; 8],
    entry_stride: u64,
    entry_stride_len: usize,
    carries_closes: bool,
}

impl Layout {
    /// Version 1 — 64-byte entries, no closes. **Read, never written.**
    ///
    /// Not retired and not refused: 43,422 entries across two vendors are on
    /// disk in this geometry and `CLAUDE.md` §3 rule 8 does not admit mutating
    /// them in place. They read at their own stride and report their closes as
    /// [`Closes::UNKNOWN`], which is the honest answer — nothing ever recorded
    /// one.
    pub const V1: Self = Self::declared(1, MAGIC, V1_ENTRY_STRIDE, V1_ENTRY_STRIDE_LEN, false);

    /// Version 2 — 128-byte entries carrying the month's first and last close.
    pub const V2: Self = Self::declared(2, MAGIC_V2, ENTRY_STRIDE, ENTRY_LEN, true);

    /// Every version this build can read, in ascending order.
    pub const KNOWN: &'static [Self] = &[Self::V1, Self::V2];

    /// The version this build writes. Older versions are read, never written.
    pub const CURRENT: Self = Self::V2;

    /// Declares a version's geometry at compile time.
    ///
    /// This is how a row of [`Layout::KNOWN`] is written. It applies exactly the
    /// rule [`Layout::declare`] applies, but as a `const` assertion, so a
    /// degenerate row does not compile rather than failing at a call site that
    /// may never run.
    ///
    /// # Panics
    ///
    /// At **compile time**, when the geometry is one no file could have. The
    /// message does not name the field, because a `const` panic cannot format
    /// one; [`Layout::declare`] does.
    #[must_use]
    pub const fn declared(
        version: u16,
        magic: [u8; 8],
        entry_stride: u64,
        entry_stride_len: usize,
        carries_closes: bool,
    ) -> Self {
        assert!(
            degenerate_field(version, magic, entry_stride, entry_stride_len).is_none(),
            "a manifest Layout row is not a geometry a file can have; see Layout::declare",
        );
        Self {
            version,
            magic,
            entry_stride,
            entry_stride_len,
            carries_closes,
        }
    }

    /// Declares a version's geometry, refusing a degenerate one.
    ///
    /// The fallible twin of [`Layout::declared`], for a geometry that is not a
    /// compile-time constant. The additive-property test builds its hypothetical
    /// next version through this door, so it drives the same [`Layout::resolve`]
    /// the read path drives rather than a paraphrase of it.
    ///
    /// # Errors
    ///
    /// [`ManifestError::DegenerateLayout`] naming the field, for a zero version,
    /// a zero stride in either width, a stride whose region at [`MAX_ENTRIES`]
    /// would overflow `u64`, or a magic outside [`MAGIC_FAMILY`].
    pub const fn declare(
        version: u16,
        magic: [u8; 8],
        entry_stride: u64,
        entry_stride_len: usize,
        carries_closes: bool,
    ) -> Result<Self, ManifestError> {
        match degenerate_field(version, magic, entry_stride, entry_stride_len) {
            Some(field) => Err(ManifestError::DegenerateLayout { field }),
            None => Ok(Self {
                version,
                magic,
                entry_stride,
                entry_stride_len,
                carries_closes,
            }),
        }
    }

    /// The layout for `version`, or a refusal naming the version.
    ///
    /// # Errors
    ///
    /// [`ManifestError::UnknownVersion`] carrying the version it saw. There is
    /// no fallback to [`Layout::CURRENT`]: a version this build does not know
    /// has a geometry this build cannot infer, and inferring it wrongly returns
    /// plausible nonsense rather than an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::manifest::{Layout, ManifestError};
    /// assert_eq!(Layout::for_version(1)?.entry_stride(), 64);
    /// assert_eq!(Layout::for_version(2)?.entry_stride(), 128);
    /// assert_eq!(Layout::for_version(9), Err(ManifestError::UnknownVersion(9)));
    /// # Ok::<(), ManifestError>(())
    /// ```
    pub fn for_version(version: u16) -> Result<Self, ManifestError> {
        Self::resolve(Self::KNOWN, version)
    }

    /// Resolves `version` against an explicit table.
    ///
    /// [`Layout::for_version`] is this function applied to [`Layout::KNOWN`]. It
    /// is public so a test can prove the additive property against a table that
    /// already holds a third version, running the same code the read path runs.
    ///
    /// # Errors
    ///
    /// [`ManifestError::UnknownVersion`].
    pub fn resolve(table: &[Self], version: u16) -> Result<Self, ManifestError> {
        table
            .iter()
            .copied()
            .find(|layout| layout.version == version)
            .ok_or(ManifestError::UnknownVersion(version))
    }

    /// This layout's version number.
    #[must_use]
    pub const fn version(self) -> u16 {
        self.version
    }

    /// The eight magic bytes a file of this version begins with.
    #[must_use]
    pub const fn magic(self) -> [u8; 8] {
        self.magic
    }

    /// Bytes per entry, in the width an offset is computed in.
    #[must_use]
    pub const fn entry_stride(self) -> u64 {
        self.entry_stride
    }

    /// Bytes per entry, in the width a slice is chunked at.
    ///
    /// Paired literals rather than a cast of the other, exactly as
    /// [`HEADER_LEN`] and its `usize` twin are: this workspace denies a cast
    /// that can truncate, and `usize` is not `u64` on every target Rust
    /// defines. The pairing is checked rather than believed —
    /// `pull::unit::every_known_manifest_layout_states_one_stride_in_both_widths`.
    #[must_use]
    pub const fn entry_stride_len(self) -> usize {
        self.entry_stride_len
    }

    /// Whether an entry of this version carries the month's closes.
    ///
    /// False for version 1, and that is not a defect to be papered over: those
    /// entries never had a close recorded, so they answer [`Closes::UNKNOWN`]
    /// rather than a zero somebody could mistake for a price.
    #[must_use]
    pub const fn carries_closes(self) -> bool {
        self.carries_closes
    }

    /// The byte offset of entry `ordinal`, for an ordinal already known to be
    /// within [`MAX_ENTRIES`].
    ///
    /// Total, not fallible. `MAX_ENTRIES · entry_stride + HEADER_LEN` is 268 MB
    /// at the widest stride this build declares, so past the bound check there
    /// is nothing left that can overflow — and therefore nothing left that needs
    /// a failure arm. [`Layout::declare`] is what keeps that true for a row
    /// added later.
    #[must_use]
    pub const fn offset_within_bounds(self, ordinal: u64) -> u64 {
        HEADER_LEN + ordinal * self.entry_stride
    }

    /// The byte offset of entry `ordinal`.
    ///
    /// `HEADER_LEN + ordinal·stride`. An add and a multiply — law 4. There is no
    /// index to consult and nothing to search.
    ///
    /// # Errors
    ///
    /// [`ManifestError::OrdinalOutOfRange`] past [`MAX_ENTRIES`]. The bound is
    /// on the ordinal rather than on the product, so the arithmetic itself is
    /// total.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::manifest::{HEADER_LEN, Layout, ManifestError, MAX_ENTRIES};
    /// assert_eq!(Layout::V1.offset_of(1)?, HEADER_LEN + 64);
    /// assert_eq!(Layout::V2.offset_of(1)?, HEADER_LEN + 128);
    /// assert!(Layout::V2.offset_of(MAX_ENTRIES + 1).is_err());
    /// # Ok::<(), ManifestError>(())
    /// ```
    pub const fn offset_of(self, ordinal: u64) -> Result<u64, ManifestError> {
        if ordinal > MAX_ENTRIES {
            return Err(ManifestError::OrdinalOutOfRange {
                ordinal,
                limit: MAX_ENTRIES,
            });
        }
        Ok(self.offset_within_bounds(ordinal))
    }

    /// How many whole entries of this version `entries` holds, capped at
    /// [`MAX_ENTRIES`].
    ///
    /// **Computed per version rather than once for the file**, because the
    /// answer is the byte length divided by a stride that the header has not
    /// been read yet to know. Counting whole entries at version 1's stride and
    /// then validating a version-2 counter against it would accept a counter
    /// naming twice the entries the region can hold.
    ///
    /// Counted by folding rather than by `len()` so the answer is a `u64`
    /// without a `usize` conversion whose failure arm no test on a 64-bit host
    /// could reach, and bounded so the fold cannot overflow.
    #[must_use]
    pub fn capacity_for(self, entries: &[u8]) -> u64 {
        entries
            .chunks_exact(self.entry_stride_len)
            .take(MAX_ENTRIES_LEN)
            .fold(0u64, |seen, _| seen + 1)
    }
}

const _: () = assert!(Layout::CURRENT.version() == FORMAT_VERSION);
const _: () = assert!(Layout::CURRENT.entry_stride() == ENTRY_STRIDE);
const _: () = assert!(Layout::V1.entry_stride() == V1_ENTRY_STRIDE);
const _: () = assert!(!Layout::V1.carries_closes() && Layout::V2.carries_closes());

/// The first field of a declaration that is not a geometry a file can have.
///
/// One function so [`Layout::declared`] and [`Layout::declare`] cannot drift: a
/// guard added here is a compile error for the const rows and an `Err` for the
/// fallible door, in the same commit.
const fn degenerate_field(
    version: u16,
    magic: [u8; 8],
    entry_stride: u64,
    entry_stride_len: usize,
) -> Option<&'static str> {
    if version == 0 {
        return Some("version");
    }
    if entry_stride == 0 {
        return Some("entry_stride");
    }
    if entry_stride_len == 0 {
        return Some("entry_stride_len");
    }
    // `HEADER_LEN + MAX_ENTRIES·stride` must fit a `u64`, because
    // `Layout::offset_within_bounds` is total past the ordinal check and has no
    // failure arm to take.
    //
    // Written as ONE comparison against [`MAX_ENTRY_STRIDE`] rather than as a
    // `checked_mul` followed by a `checked_add`: at this [`MAX_ENTRIES`] the
    // second of those could never fire — the products step by 2,097,152 and the
    // header region is 32,768, so a product that fits always leaves room for it
    // — and an arm no input can enter is an arm no test can close.
    if entry_stride > MAX_ENTRY_STRIDE {
        return Some("entry_stride");
    }
    if !magic_is_family(magic) {
        return Some("magic");
    }
    None
}

/// Whether `magic`'s first seven bytes are [`MAGIC_FAMILY`].
///
/// Destructured rather than iterated because this runs in `const` context, where
/// an iterator is not available and an index would be a denied lint.
const fn magic_is_family(magic: [u8; 8]) -> bool {
    let [m0, m1, m2, m3, m4, m5, m6, _] = magic;
    let [f0, f1, f2, f3, f4, f5, f6] = MAGIC_FAMILY;
    m0 == f0 && m1 == f1 && m2 == f2 && m3 == f3 && m4 == f4 && m5 == f5 && m6 == f6
}

/// Why one entry's bytes are not an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntryFault {
    /// Fewer than [`IMAGE_LEN`] bytes were offered.
    TooShort {
        /// Bytes the caller supplied.
        len: usize,
    },
    /// The stored checksum does not match the bytes.
    Checksum {
        /// The checksum the entry carries.
        stored: u32,
        /// The checksum its bytes actually produce.
        computed: u32,
    },
    /// The symbol field is not UTF-8.
    SymbolNotUtf8,
    /// The symbol field is not a symbol.
    BadSymbol(InstrumentError),
    /// The symbol field is not in the canonical form a [`Symbol`] renders.
    ///
    /// Refused rather than folded, and for the reason `store::path` gives about
    /// case: an entry written as `nifty` and read back as `NIFTY` is a key that
    /// does not match itself, so the census would count one month twice.
    SymbolNotCanonical,
    /// The exchange code is not one this build defines.
    UnknownExchange {
        /// The code that was found.
        code: u8,
    },
    /// The segment code is not one this build defines.
    UnknownSegment {
        /// The code that was found.
        code: u8,
    },
    /// No timeframe of that length exists — `store::path::Timeframe::KNOWN`.
    UnknownTimeframe {
        /// The length found, in seconds.
        secs: u32,
    },
    /// The year and month are not a month.
    BadMonth(PathError),
    /// The entry records no rows.
    ///
    /// A manifest is a census of what exists. An entry claiming zero rows
    /// describes a file with nothing in it, which is a write that should never
    /// have been recorded rather than a fact to be stored.
    EmptyEntry,
    /// The entry's own timestamps run backwards.
    TimestampsOutOfOrder {
        /// The first timestamp.
        first: i64,
        /// The last timestamp, which did not follow it.
        last: i64,
    },
    /// One close is [`CLOSE_NULL`] and the other is a price.
    ///
    /// A month has both closes or neither: they are read from record 0 and
    /// record `n_valid − 1` of one file in one operation. Half of a pair is a
    /// state no writer here can produce, and reading it as either not-recorded
    /// or recorded would be inventing the missing half.
    CloseHalfRecorded {
        /// The first close as stored.
        first: i64,
        /// The last close as stored.
        last: i64,
    },
    /// A close is below zero and is not [`CLOSE_NULL`].
    ///
    /// Prices are paisa integers and no tick grid produces a negative one, so
    /// the sentinel is the **only** negative value this field may hold. Refusing
    /// the rest is what keeps not-recorded a single value rather than a range.
    CloseNotAPrice {
        /// The value that is neither a price nor the sentinel.
        paisa: i64,
    },
}

impl fmt::Display for EntryFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooShort { len } => write!(f, "entry is {len} bytes, needs {IMAGE_LEN}"),
            Self::Checksum { stored, computed } => {
                write!(f, "entry checksum {stored:#010x} != {computed:#010x}")
            }
            Self::SymbolNotUtf8 => f.write_str("the symbol field is not UTF-8"),
            Self::BadSymbol(e) => write!(f, "the symbol field is not a symbol: {e}"),
            Self::SymbolNotCanonical => f.write_str("the symbol field is not in canonical form"),
            Self::UnknownExchange { code } => write!(f, "exchange code {code} is not defined"),
            Self::UnknownSegment { code } => write!(f, "segment code {code} is not defined"),
            Self::UnknownTimeframe { secs } => write!(f, "no timeframe of {secs} seconds"),
            Self::BadMonth(e) => write!(f, "not a month: {e}"),
            Self::EmptyEntry => f.write_str("the entry records no rows"),
            Self::TimestampsOutOfOrder { first, last } => {
                write!(f, "timestamp {last} does not follow {first}")
            }
            Self::CloseHalfRecorded { first, last } => write!(
                f,
                "closes ({first}, {last}): one is the not-recorded sentinel and \
                 the other is a price; a month has both or neither"
            ),
            Self::CloseNotAPrice { paisa } => write!(
                f,
                "close {paisa} paisa is neither a price nor the not-recorded \
                 sentinel {CLOSE_NULL}"
            ),
        }
    }
}

impl std::error::Error for EntryFault {}

/// Why a manifest was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ManifestError {
    /// An entry ordinal is past [`MAX_ENTRIES`].
    ///
    /// Refused rather than allowed to wrap. Bounding the *ordinal* rather than
    /// checking the multiplication afterwards is what lets every offset
    /// computed inside this module be plain arithmetic: past this gate the
    /// product cannot overflow, so there is no failure arm downstream for a
    /// test to be unable to reach.
    OrdinalOutOfRange {
        /// The ordinal that was refused.
        ordinal: u64,
        /// The bound.
        limit: u64,
    },
    /// A header slot buffer is shorter than [`IMAGE_LEN`].
    SlotTooShort {
        /// Bytes the caller supplied.
        len: usize,
    },
    /// The first seven bytes are not [`MAGIC_FAMILY`].
    NotAManifest,
    /// The slot names a version this build does not know.
    UnknownVersion(u16),
    /// The slot names an entry stride its version does not define.
    StrideMismatch(u16),
    /// The slot's stored checksum does not match its bytes.
    SlotChecksum {
        /// The checksum the slot carries.
        stored: u32,
        /// The checksum its bytes actually produce.
        computed: u32,
    },
    /// The slot's vendor field is not a vendor this build reads.
    UnknownVendor,
    /// The manifest belongs to a different vendor than the one asked for.
    ///
    /// The file name carries the vendor and so does the header; when they
    /// disagree, one of them is a typo and appending would file one vendor's
    /// census under another's name.
    VendorMismatch {
        /// The vendor asked for.
        asked: Vendor,
        /// The vendor the header names.
        found: Vendor,
    },
    /// A slot's generation says it belongs in a different slot.
    SlotPositionMismatch {
        /// Where `generation % SLOT_COUNT` says it belongs.
        expected: u64,
        /// Where it was found.
        found: u64,
    },
    /// The header region is shorter than [`SLOT_COUNT`] whole slots.
    HeaderRegionTooShort {
        /// Whole slots the region actually holds.
        slots: u64,
        /// Whole slots this version declares.
        need: u64,
    },
    /// No slot in the header region decoded.
    NoValidHeader,
    /// `n_valid` claims more entries than [`MAX_ENTRIES`].
    TooManyEntries {
        /// The counter that was refused.
        n_valid: u64,
        /// The bound.
        limit: u64,
    },
    /// `n_valid` claims more entries than the region can hold.
    CounterExceedsRegion {
        /// The counter that was refused.
        n_valid: u64,
        /// Whole entries the region holds.
        capacity: u64,
    },
    /// `n_keys` claims more distinct keys than there are entries.
    KeyCountExceedsEntries {
        /// Distinct keys claimed.
        keys: u64,
        /// Entries claimed.
        entries: u64,
    },
    /// Appending would push `n_valid` past `u64::MAX`.
    CounterOverflow,
    /// The generation counter cannot be advanced again.
    ///
    /// Refused rather than wrapped: a wrapped generation makes the *oldest*
    /// slot win and silently un-commits entries.
    GenerationExhausted,
    /// The row total would pass `u64::MAX`.
    RowTotalOverflow,
    /// One entry could not be decoded.
    Entry {
        /// Which entry.
        ordinal: u64,
        /// What was wrong with it.
        fault: EntryFault,
    },
    /// A later entry for one key records fewer rows than an earlier one.
    ///
    /// `CLAUDE.md` §3 rule 8 — history is append-only. A month whose row count
    /// went down is a re-pull that lost data, and the manifest is the only
    /// place that fact is visible without re-reading the bar file.
    RowCountWentBackwards {
        /// Which entry.
        ordinal: u64,
        /// Rows the earlier entry recorded.
        previous: u64,
        /// Rows this entry records.
        next: u64,
    },
    /// A later entry for one key ends before an earlier one did.
    KeyTimestampsOutOfOrder {
        /// Which entry.
        ordinal: u64,
        /// The last timestamp already recorded.
        previous: i64,
        /// The timestamp that did not follow it.
        next: i64,
    },
    /// The header's key count is not the number of distinct keys present.
    KeyCountDisagrees {
        /// What the header claims.
        header: u64,
        /// What the entries hold.
        entries: u64,
    },
    /// The header's row total is not the sum of the entries' row counts.
    RowTotalDisagrees {
        /// What the header claims.
        header: u64,
        /// What the entries hold.
        entries: u64,
    },
    /// A [`Layout`] row is not a geometry a manifest could have.
    DegenerateLayout {
        /// Which field.
        field: &'static str,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OrdinalOutOfRange { ordinal, limit } => {
                write!(
                    f,
                    "entry ordinal {ordinal} is past the {limit} this build holds"
                )
            }
            Self::SlotTooShort { len } => {
                write!(f, "header slot is {len} bytes, needs {IMAGE_LEN}")
            }
            Self::NotAManifest => f.write_str("magic does not begin BRUTEXM"),
            Self::UnknownVersion(v) => write!(f, "unknown manifest version {v}"),
            Self::StrideMismatch(s) => write!(f, "entry stride {s} is not this version's stride"),
            Self::SlotChecksum { stored, computed } => {
                write!(f, "header slot checksum {stored:#010x} != {computed:#010x}")
            }
            Self::UnknownVendor => f.write_str("the header names no vendor this build reads"),
            Self::VendorMismatch { asked, found } => write!(
                f,
                "this manifest belongs to {}, not {}",
                found.as_str(),
                asked.as_str()
            ),
            Self::SlotPositionMismatch { expected, found } => write!(
                f,
                "header slot {found} holds a commit belonging in {expected}"
            ),
            Self::HeaderRegionTooShort { slots, need } => {
                write!(f, "header region holds {slots} whole slots, needs {need}")
            }
            Self::NoValidHeader => f.write_str("no header slot survived; the header is unreadable"),
            Self::TooManyEntries { n_valid, limit } => {
                write!(f, "n_valid is {n_valid}; this build holds at most {limit}")
            }
            Self::CounterExceedsRegion { n_valid, capacity } => {
                write!(f, "n_valid is {n_valid}; the region holds {capacity}")
            }
            Self::KeyCountExceedsEntries { keys, entries } => {
                write!(f, "{keys} distinct keys across {entries} entries")
            }
            Self::CounterOverflow => f.write_str("n_valid would overflow u64"),
            Self::GenerationExhausted => f.write_str("generation cannot advance past u64"),
            Self::RowTotalOverflow => f.write_str("the row total would overflow u64"),
            Self::Entry { ordinal, fault } => write!(f, "entry {ordinal}: {fault}"),
            Self::RowCountWentBackwards {
                ordinal,
                previous,
                next,
            } => write!(
                f,
                "entry {ordinal} records {next} rows where {previous} were already recorded"
            ),
            Self::KeyTimestampsOutOfOrder {
                ordinal,
                previous,
                next,
            } => write!(
                f,
                "entry {ordinal}: timestamp {next} does not follow {previous}"
            ),
            Self::KeyCountDisagrees { header, entries } => write!(
                f,
                "the header claims {header} distinct keys; the entries hold {entries}"
            ),
            Self::RowTotalDisagrees { header, entries } => write!(
                f,
                "the header claims {header} rows; the entries hold {entries}"
            ),
            Self::DegenerateLayout { field } => {
                write!(f, "{field} is not a geometry a manifest could have")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// The exchange's code on disk.
///
/// Append-only, like every other number this repository writes down: a code is
/// never reused for a different exchange. BSE is defined even though `CLAUDE.md`
/// §1 sweeps and pulls NSE alone, because BSE data already on disk is not
/// deleted and a census that could not name it would under-report the store.
fn exchange_code(exchange: Exchange) -> u8 {
    match exchange {
        Exchange::Nse => 1,
        Exchange::Bse => 2,
    }
}

/// The exchange a code names.
fn exchange_of(code: u8) -> Result<Exchange, EntryFault> {
    match code {
        1 => Ok(Exchange::Nse),
        2 => Ok(Exchange::Bse),
        _ => Err(EntryFault::UnknownExchange { code }),
    }
}

/// The segment's code on disk. Append-only.
fn segment_code(segment: Segment) -> u8 {
    match segment {
        Segment::Index => 1,
        Segment::Cash => 2,
        Segment::Fno => 3,
    }
}

/// The segment a code names.
fn segment_of(code: u8) -> Result<Segment, EntryFault> {
    match code {
        1 => Ok(Segment::Index),
        2 => Ok(Segment::Cash),
        3 => Ok(Segment::Fno),
        _ => Err(EntryFault::UnknownSegment { code }),
    }
}

/// What one manifest entry is keyed by.
///
/// Exactly the tuple that names one bar file, minus the vendor, which the
/// manifest itself carries. Structural `Eq` and `Hash` over every field, so it
/// is directly a `HashMap` key and a lookup is one probe rather than a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryKey {
    /// Trading venue.
    pub exchange: Exchange,
    /// Exchange segment.
    pub segment: Segment,
    /// The symbol, fixed width — `docs/07-o1-architecture.md` layer 1.
    pub symbol: Symbol,
    /// The bar length.
    pub timeframe: Timeframe,
    /// The month the bar file covers.
    pub month: YearMonth,
}

/// The month's first and last close, or the fact that neither was recorded.
///
/// # Why the census carries a price at all
///
/// `/store.json` serves one row per `(instrument, month)` and the operator asks
/// it for the month's percentage change. Deriving that from the bars costs one
/// request and a whole month of records per row — measured at 31,951 requests
/// and ~17.5 GB to extract 492 KB of signal, and it grows with the census. Two
/// `i64` beside the row answer it in one probe. D-0067.
///
/// # The fields are private, and that is the invariant
///
/// A `Closes` is **either** both-unknown or both-a-price. Half of a pair, or a
/// negative that is not [`CLOSE_NULL`], is unrepresentable outside this module
/// — [`Closes::known`] is the only door and it refuses both. That is what makes
/// [`Closes::is_known`] a single comparison rather than a rule a caller has to
/// remember, and it is why the encoder and the decoder cannot drift: they go
/// through the same door.
///
/// # What it does NOT say
///
/// Nothing about corporate actions. An equity that split 1:5 mid-month has two
/// closes that are both real and a ratio between them that is a fabricated 80%
/// crash. This type records the two prices; whether a percentage may be
/// computed from them is a separate decision, and `docs/05-decisions.md` D-0018
/// already made it — refuse loudly and name the date, never back-adjust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Closes {
    /// The close of record 0, or [`CLOSE_NULL`].
    first_paisa: i64,
    /// The close of record `rows − 1`, or [`CLOSE_NULL`].
    last_paisa: i64,
}

impl Closes {
    /// No close was recorded for this month. **Not zero, and never rendered as
    /// one.**
    ///
    /// Every version-1 entry reads back as this, because version 1 had nowhere
    /// to put a close. So does a version-2 entry a writer chose not to price.
    pub const UNKNOWN: Self = Self {
        first_paisa: CLOSE_NULL,
        last_paisa: CLOSE_NULL,
    };

    /// The closes of a month whose bar file was read.
    ///
    /// # Errors
    ///
    /// [`EntryFault::CloseNotAPrice`] for a value below zero — which includes
    /// [`CLOSE_NULL`] itself, so not-recorded cannot be smuggled in through the
    /// door marked [`Closes::known`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::manifest::{CLOSE_NULL, Closes};
    /// let held = Closes::known(2_950_50, 3_100_25)?;
    /// assert_eq!(held.paisa(), Some((295_050, 310_025)));
    /// // Zero is a price. It is NOT unknown.
    /// assert_eq!(Closes::known(0, 0)?.paisa(), Some((0, 0)));
    /// assert_eq!(Closes::UNKNOWN.paisa(), None);
    /// assert!(Closes::known(CLOSE_NULL, 1).is_err());
    /// # Ok::<(), pull::manifest::EntryFault>(())
    /// ```
    pub const fn known(first_paisa: i64, last_paisa: i64) -> Result<Self, EntryFault> {
        if first_paisa < 0 {
            return Err(EntryFault::CloseNotAPrice { paisa: first_paisa });
        }
        if last_paisa < 0 {
            return Err(EntryFault::CloseNotAPrice { paisa: last_paisa });
        }
        Ok(Self {
            first_paisa,
            last_paisa,
        })
    }

    /// Whether a close was recorded at all.
    ///
    /// One comparison, and it may look at either field: [`Closes::known`]
    /// refuses every negative, so [`CLOSE_NULL`] appears in one field only when
    /// it appears in both.
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.first_paisa != CLOSE_NULL
    }

    /// The month's first close in paisa, or `None` when none was recorded.
    #[must_use]
    pub const fn first_paisa(self) -> Option<i64> {
        if self.is_known() {
            Some(self.first_paisa)
        } else {
            None
        }
    }

    /// The month's last close in paisa, or `None` when none was recorded.
    #[must_use]
    pub const fn last_paisa(self) -> Option<i64> {
        if self.is_known() {
            Some(self.last_paisa)
        } else {
            None
        }
    }

    /// Both closes, or `None` when neither was recorded.
    ///
    /// The shape a caller computing a percentage wants: there is no arm in
    /// which one is available and the other is not, so there is no arm in which
    /// a caller can accidentally divide by a number it invented.
    #[must_use]
    pub const fn paisa(self) -> Option<(i64, i64)> {
        if self.is_known() {
            Some((self.first_paisa, self.last_paisa))
        } else {
            None
        }
    }

    /// The pair as stored, refusing a half-recorded or impossible one.
    ///
    /// The decoder's door. It is the same door the encoder used, which is what
    /// makes the round trip a property of the type rather than of two functions
    /// that happen to agree today.
    const fn decode(first: i64, last: i64) -> Result<Self, EntryFault> {
        match (first == CLOSE_NULL, last == CLOSE_NULL) {
            (true, true) => Ok(Self::UNKNOWN),
            (false, false) => Self::known(first, last),
            // Exactly one sentinel. Reading it as either answer invents the
            // other half, and `CLAUDE.md` §4 admits no fallback that hides a
            // failure.
            (true, false) | (false, true) => Err(EntryFault::CloseHalfRecorded { first, last }),
        }
    }

    /// The two values as they go on disk, sentinel and all.
    const fn stored(self) -> (i64, i64) {
        (self.first_paisa, self.last_paisa)
    }
}

/// One month of one instrument, as the manifest records it.
///
/// **The version-1 record, unchanged.** Its four fields are at the offsets
/// version 1 wrote them at and its image is still 64 bytes, because
/// `CLAUDE.md` §3 rule 8 makes a format append-only: version 2 adds a second
/// 64-byte half beside this one rather than renumbering it. [`Held`] is an
/// entry together with the closes that half carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entry {
    /// What this entry is about.
    pub key: EntryKey,
    /// Rows in that bar file.
    pub rows: u64,
    /// Timestamp of the first row.
    pub first_ts_micros: i64,
    /// Timestamp of the last row.
    pub last_ts_micros: i64,
}

impl Entry {
    /// Whether this entry is internally consistent.
    ///
    /// Run before an entry is written **and** after one is read, because the
    /// checksum proves the bytes are the bytes that were written and says
    /// nothing about whether they were ever sensible.
    ///
    /// # Errors
    ///
    /// [`EntryFault::EmptyEntry`] or [`EntryFault::TimestampsOutOfOrder`].
    pub const fn check(&self) -> Result<(), EntryFault> {
        if self.rows == 0 {
            return Err(EntryFault::EmptyEntry);
        }
        if self.last_ts_micros < self.first_ts_micros {
            return Err(EntryFault::TimestampsOutOfOrder {
                first: self.first_ts_micros,
                last: self.last_ts_micros,
            });
        }
        Ok(())
    }

    /// This entry's image at the version this build writes — [`ENTRY_LEN`]
    /// bytes, with the closes recorded as [`Closes::UNKNOWN`].
    ///
    /// A convenience for the caller that has an entry and no price for it.
    /// [`Held::image`] is the one that carries a close.
    #[must_use]
    pub fn image(&self) -> [u8; ENTRY_LEN] {
        Held::unknown(*self).image()
    }

    /// The **version-1** image of this entry: 64 bytes, checksum computed.
    ///
    /// Also bytes `0..64` of the version-2 image, byte for byte — which is not
    /// a coincidence to be relied on quietly but the shape of the format:
    /// version 2 is two of this module's 64-byte checksummed units, the first
    /// of which is a version-1 entry. `pull::unit::a_version_2_entry_opens_with_a_version_1_entry`
    /// pins it.
    #[must_use]
    pub fn image_v1(&self) -> [u8; IMAGE_LEN] {
        let mut out = [0u8; IMAGE_LEN];
        write_text(&mut out, E_SYMBOL, self.key.symbol.as_str());
        write_at(&mut out, E_ROWS, self.rows.to_le_bytes());
        write_at(&mut out, E_FIRST_TS, self.first_ts_micros.to_le_bytes());
        write_at(&mut out, E_LAST_TS, self.last_ts_micros.to_le_bytes());
        write_at(
            &mut out,
            E_TIMEFRAME,
            self.key.timeframe.secs().to_le_bytes(),
        );
        write_at(&mut out, E_YEAR, self.key.month.year().to_le_bytes());
        write_at(&mut out, E_MONTH, self.key.month.month().to_le_bytes());
        write_at(
            &mut out,
            E_EXCHANGE,
            exchange_code(self.key.exchange).to_le_bytes(),
        );
        write_at(
            &mut out,
            E_SEGMENT,
            segment_code(self.key.segment).to_le_bytes(),
        );
        seal(&mut out);
        out
    }

    /// Decodes the **base** 64 bytes of an entry — a whole version-1 entry, or
    /// the first half of a version-2 one.
    ///
    /// It reads no close and refuses none: a version-2 entry's second half is
    /// [`Held::decode`]'s to check. A caller holding version-2 bytes and calling
    /// this gets the four base fields and no error, which is correct and is not
    /// the whole row — [`Layout::decode_entry`] is the door that dispatches.
    ///
    /// The bytes are copied into a 64-byte array **once**, before anything is
    /// checked, and every field is read from that copy — the same argument
    /// `store::header::Header::decode` makes: the checksum must cover exactly
    /// the bytes the returned value is built from, or a concurrent write
    /// produces a value no checksum ever saw.
    ///
    /// # Errors
    ///
    /// [`EntryFault`], checked in this order: length, checksum, then each field.
    /// The checksum first, because a field refusal from bytes that already
    /// failed their checksum is a diagnosis of corruption as a schema problem.
    pub fn decode(bytes: &[u8]) -> Result<Self, EntryFault> {
        let image = image_of(bytes).map_err(|len| EntryFault::TooShort { len })?;
        verify(&image).map_err(|(stored, computed)| EntryFault::Checksum { stored, computed })?;

        let text = text_at(&image, E_SYMBOL, SYMBOL_CAPACITY).ok_or(EntryFault::SymbolNotUtf8)?;
        let symbol = Symbol::new(text).map_err(EntryFault::BadSymbol)?;
        if symbol.as_str() != text {
            return Err(EntryFault::SymbolNotCanonical);
        }
        let secs = u32::from_le_bytes(le_bytes(&image, E_TIMEFRAME));
        let timeframe =
            Timeframe::from_secs(secs).map_err(|_| EntryFault::UnknownTimeframe { secs })?;
        let year = u16::from_le_bytes(le_bytes(&image, E_YEAR));
        let [month] = le_bytes(&image, E_MONTH);
        let month = YearMonth::new(year, month).map_err(EntryFault::BadMonth)?;
        let [exchange] = le_bytes(&image, E_EXCHANGE);
        let [segment] = le_bytes(&image, E_SEGMENT);

        let entry = Self {
            key: EntryKey {
                exchange: exchange_of(exchange)?,
                segment: segment_of(segment)?,
                symbol,
                timeframe,
                month,
            },
            rows: u64::from_le_bytes(le_bytes(&image, E_ROWS)),
            first_ts_micros: i64::from_le_bytes(le_bytes(&image, E_FIRST_TS)),
            last_ts_micros: i64::from_le_bytes(le_bytes(&image, E_LAST_TS)),
        };
        entry.check()?;
        Ok(entry)
    }
}

/// One row of the census: the entry, and the closes recorded beside it.
///
/// # Why this is a second type and not two more fields on [`Entry`]
///
/// Because that is what the bytes are. A version-2 entry is **two** 64-byte
/// checksummed units — a version-1 entry, then the closes — and this type is
/// that pair with the same seam in the same place. `Entry` keeps meaning "the
/// version-1 record", so nothing that already reads or writes one changes
/// meaning, and a version-1 file loads into `Held { entry, closes: UNKNOWN }`
/// without a special case anywhere.
///
/// It is also what makes the ingest probe correct. `pull::ingest::count` asks
/// "is this month already recorded exactly as it is now" with one comparison,
/// and if that comparison were over `Entry` alone, a month whose rows had not
/// moved but whose closes had just been read for the first time would compare
/// equal and the closes would never be written. Comparing the whole row is what
/// makes the version-1 census fill in as it is re-ingested — and, once filled,
/// stop rewriting, because then the closes compare equal too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Held {
    /// What the census records about the bar file.
    pub entry: Entry,
    /// The month's first and last close, or [`Closes::UNKNOWN`].
    pub closes: Closes,
}

impl Held {
    /// A row whose closes nobody has read.
    #[must_use]
    pub const fn unknown(entry: Entry) -> Self {
        Self {
            entry,
            closes: Closes::UNKNOWN,
        }
    }

    /// A row with both parts.
    #[must_use]
    pub const fn new(entry: Entry, closes: Closes) -> Self {
        Self { entry, closes }
    }

    /// The version-2 image of this row: [`ENTRY_LEN`] bytes, both checksums
    /// computed.
    ///
    /// Bytes `0..64` are exactly [`Entry::image_v1`]. Bytes `64..128` are their
    /// own 64-byte image — the two closes, then reserved zeroes, then a CRC-32C
    /// over the first 60 of them at the same offset the other half uses. So
    /// every one of the 128 bytes is covered by exactly one checksum, and the
    /// same [`seal`] serves both halves rather than a second kernel that could
    /// disagree with the first.
    #[must_use]
    pub fn image(&self) -> [u8; ENTRY_LEN] {
        let base = self.entry.image_v1();

        let mut closes = [0u8; IMAGE_LEN];
        let (first, last) = self.closes.stored();
        write_at(&mut closes, C_FIRST_CLOSE, first.to_le_bytes());
        write_at(&mut closes, C_LAST_CLOSE, last.to_le_bytes());
        // 16..60 stays zero. `docs/02-store-format.md` §2: a future field takes
        // reserved space in a NEW VERSION.
        seal(&mut closes);

        let mut out = [0u8; ENTRY_LEN];
        for (dst, src) in out.iter_mut().zip(base.iter().chain(closes.iter())) {
            *dst = *src;
        }
        out
    }

    /// Decodes a version-2 entry: both halves, both checksums, both fields.
    ///
    /// # Errors
    ///
    /// [`EntryFault::TooShort`] for fewer than [`ENTRY_LEN`] bytes, whatever
    /// [`Entry::decode`] refuses in the first half, [`EntryFault::Checksum`] for
    /// a second half that fails its own, or [`EntryFault::CloseHalfRecorded`] /
    /// [`EntryFault::CloseNotAPrice`] for a pair that is not a pair.
    pub fn decode(bytes: &[u8]) -> Result<Self, EntryFault> {
        let entry = Entry::decode(bytes)?;
        let half = image_of_at(bytes, IMAGE_LEN).map_err(|len| EntryFault::TooShort { len })?;
        verify(&half).map_err(|(stored, computed)| EntryFault::Checksum { stored, computed })?;
        let closes = Closes::decode(
            i64::from_le_bytes(le_bytes(&half, C_FIRST_CLOSE)),
            i64::from_le_bytes(le_bytes(&half, C_LAST_CLOSE)),
        )?;
        Ok(Self { entry, closes })
    }
}

impl Layout {
    /// Decodes one entry **at this version's geometry**.
    ///
    /// This is the dispatch the module header is about: a version-1 entry is 64
    /// bytes with no close, a version-2 entry is 128 with two, and reading
    /// either at the other's stride returns plausible integers lifted from the
    /// wrong offsets.
    ///
    /// # Errors
    ///
    /// Whatever [`Entry::decode`] or [`Held::decode`] refuses.
    pub fn decode_entry(self, bytes: &[u8]) -> Result<Held, EntryFault> {
        if self.carries_closes {
            Held::decode(bytes)
        } else {
            Entry::decode(bytes).map(Held::unknown)
        }
    }
}

/// One header slot's fields.
///
/// The three counters are the whole point of the file. They are maintained on
/// every write and checked against the entries on every load, so reading one is
/// a field read that means something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManifestHeader {
    /// Format version. **Selects the geometry** — see [`Layout`]. Refused when
    /// unknown, never read at the current version's offsets.
    pub format_version: u16,
    /// Bytes per entry, as this file declares them.
    ///
    /// Read, and then checked against what [`Layout`] says the version's stride
    /// is. A file may not declare a stride its own version does not have.
    pub entry_stride: u16,
    /// Which vendor's census this is. A cross-check against the file name.
    pub vendor: Vendor,
    /// Which commit this slot holds. Higher wins.
    pub generation: u64,
    /// The commit counter: how many entries are readable.
    pub n_valid: u64,
    /// Distinct keys among those entries.
    pub n_keys: u64,
    /// Rows held across every distinct key — the counter law 3 is about.
    pub total_rows: u64,
}

/// One header write: one offset, one buffer, one positional write.
///
/// The type is the guarantee. A commit cannot be expressed as several field
/// updates because there is nowhere to put the second one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commit {
    /// Which slot this belongs in — `generation % SLOT_COUNT`.
    pub slot: u64,
    /// The byte offset to write at.
    pub offset: u64,
    /// Exactly the bytes to write, checksum included.
    pub bytes: [u8; IMAGE_LEN],
    /// The byte offset through which entry data must already be durable.
    ///
    /// Issuing the slot write before the bytes below this offset are on stable
    /// storage publishes a counter over entries that may not exist. This crate
    /// performs no I/O and cannot issue the barrier; it states the offset so a
    /// writer cannot claim it did not know which one to flush.
    pub durable_through: u64,
    /// The header state this commit publishes.
    pub header: ManifestHeader,
}

/// One entry write: one offset and one buffer, plus the commit that publishes
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Append {
    /// Which entry this is — the index the offset is computed from.
    pub ordinal: u64,
    /// The byte offset to write the entry at.
    pub offset: u64,
    /// Exactly the bytes to write, both checksums included.
    ///
    /// [`ENTRY_LEN`] and not [`IMAGE_LEN`]: an append is always at the version
    /// this build writes, and that entry is two 64-byte checksummed units.
    pub bytes: [u8; ENTRY_LEN],
    /// The header write that publishes it, to be issued **after** the entry is
    /// durable through [`Commit::durable_through`].
    pub commit: Commit,
}

impl ManifestHeader {
    /// A fresh header for an empty manifest: generation 0, no entries.
    #[must_use]
    pub const fn genesis(vendor: Vendor) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            entry_stride: ENTRY_STRIDE_U16,
            vendor,
            generation: 0,
            n_valid: 0,
            n_keys: 0,
            total_rows: 0,
        }
    }

    /// The geometry this header's version declares.
    ///
    /// # Errors
    ///
    /// [`ManifestError::UnknownVersion`]. The fields of this struct are public,
    /// so a header naming a version no [`Layout`] declares is constructible —
    /// and answering with [`Layout::CURRENT`] would address its entries at a
    /// stride it never claimed.
    pub fn layout(&self) -> Result<Layout, ManifestError> {
        Layout::for_version(self.format_version)
    }

    /// This header restated at the version this build writes.
    ///
    /// **The migration, and it is one line.** A census loaded from a version-1
    /// file is published at version 2, because older versions are read and never
    /// written — the same rule `store::layout::Layout::CURRENT` states. Nothing
    /// on disk is mutated by this: the version-1 bytes stay exactly as they are
    /// until a run has something new to record, and then the whole file is
    /// rewritten rather than appended to. See [`Manifest::upgrading`].
    const fn at_current(self) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            entry_stride: ENTRY_STRIDE_U16,
            ..self
        }
    }

    /// The header that publishing `appended` more entries produces.
    ///
    /// Advances the generation and the counter together, because they are
    /// written together — **and restates the version as the one this build
    /// writes**, so the first commit after a version-1 census is loaded is a
    /// version-2 commit. A header that advanced at the loaded version would
    /// publish a counter over entries this build no longer knows how to write.
    ///
    /// # Errors
    ///
    /// [`ManifestError::GenerationExhausted`] or
    /// [`ManifestError::CounterOverflow`] — both refusals, never wraps.
    /// [`ManifestError::TooManyEntries`] past [`MAX_ENTRIES`], or
    /// [`ManifestError::KeyCountExceedsEntries`] for a key count no set of
    /// entries could produce.
    pub const fn advance(
        &self,
        appended: u64,
        n_keys: u64,
        total_rows: u64,
    ) -> Result<Self, ManifestError> {
        let Some(generation) = self.generation.checked_add(1) else {
            return Err(ManifestError::GenerationExhausted);
        };
        let Some(n_valid) = self.n_valid.checked_add(appended) else {
            return Err(ManifestError::CounterOverflow);
        };
        if n_valid > MAX_ENTRIES {
            return Err(ManifestError::TooManyEntries {
                n_valid,
                limit: MAX_ENTRIES,
            });
        }
        if n_keys > n_valid {
            return Err(ManifestError::KeyCountExceedsEntries {
                keys: n_keys,
                entries: n_valid,
            });
        }
        Ok(Self {
            generation,
            n_valid,
            n_keys,
            total_rows,
            ..self.at_current()
        })
    }

    /// Checks this header against the entries a region can hold.
    ///
    /// # Errors
    ///
    /// [`ManifestError::TooManyEntries`],
    /// [`ManifestError::CounterExceedsRegion`] — trusting the counter over the
    /// region reads past the end and returns whatever was there — or
    /// [`ManifestError::KeyCountExceedsEntries`].
    pub const fn validate(&self, capacity: u64) -> Result<(), ManifestError> {
        if self.n_valid > MAX_ENTRIES {
            return Err(ManifestError::TooManyEntries {
                n_valid: self.n_valid,
                limit: MAX_ENTRIES,
            });
        }
        if self.n_valid > capacity {
            return Err(ManifestError::CounterExceedsRegion {
                n_valid: self.n_valid,
                capacity,
            });
        }
        if self.n_keys > self.n_valid {
            return Err(ManifestError::KeyCountExceedsEntries {
                keys: self.n_keys,
                entries: self.n_valid,
            });
        }
        Ok(())
    }

    /// The single write that publishes this header.
    ///
    /// # Preconditions the caller owns
    ///
    /// 1. **One writer per manifest.** Two writers that read the same
    ///    generation produce the same slot at the same offset. This crate
    ///    performs no I/O and cannot take a lock.
    /// 2. **Flush first.** Every byte below [`Commit::durable_through`] must be
    ///    on stable storage before this write is issued.
    ///
    /// # Errors
    ///
    /// [`ManifestError::TooManyEntries`] if the counter is past
    /// [`MAX_ENTRIES`], or [`ManifestError::UnknownVersion`] for a header naming
    /// a version no [`Layout`] declares — its `durable_through` is an offset,
    /// and an offset needs a stride. Past those gates every offset below is
    /// plain arithmetic that provably cannot overflow, which is why nothing
    /// downstream carries a failure arm no test could enter.
    pub fn commit(&self) -> Result<Commit, ManifestError> {
        if self.n_valid > MAX_ENTRIES {
            return Err(ManifestError::TooManyEntries {
                n_valid: self.n_valid,
                limit: MAX_ENTRIES,
            });
        }
        Ok(self.commit_at(self.layout()?))
    }

    /// [`ManifestHeader::commit`] for a header whose counter is already known
    /// to be within [`MAX_ENTRIES`] and whose geometry the caller holds.
    ///
    /// Private, and neither precondition is a comment on a public door: the only
    /// callers other than [`ManifestHeader::commit`] are [`Manifest::record_held`]
    /// and [`Manifest::image`], which reach it through
    /// [`ManifestHeader::advance`] and [`ManifestHeader::at_current`] — the
    /// first refuses a counter past [`MAX_ENTRIES`] and both leave the version
    /// at [`Layout::CURRENT`].
    fn commit_at(&self, layout: Layout) -> Commit {
        let slot = self.generation % SLOT_COUNT;
        Commit {
            slot,
            offset: slot * SLOT_STRIDE,
            bytes: self.image(),
            durable_through: layout.offset_within_bounds(self.n_valid),
            header: *self,
        }
    }

    /// The 64-byte slot image for this header, checksum computed.
    ///
    /// The magic follows the **version**, from [`Layout`], rather than a
    /// constant: a version-1 slot begins `BRUTEXM1` and a version-2 slot begins
    /// `BRUTEXM2`, so the two can never be mistaken for one another even before
    /// the version field is read.
    #[must_use]
    pub fn image(&self) -> [u8; IMAGE_LEN] {
        let mut out = [0u8; IMAGE_LEN];
        write_at(&mut out, H_MAGIC, magic_for(self.format_version));
        write_at(&mut out, H_VERSION, self.format_version.to_le_bytes());
        write_at(&mut out, H_STRIDE, self.entry_stride.to_le_bytes());
        write_at(&mut out, H_GENERATION, self.generation.to_le_bytes());
        write_at(&mut out, H_N_VALID, self.n_valid.to_le_bytes());
        write_at(&mut out, H_N_KEYS, self.n_keys.to_le_bytes());
        write_at(&mut out, H_TOTAL_ROWS, self.total_rows.to_le_bytes());
        write_text(&mut out, H_VENDOR, self.vendor.as_str());
        seal(&mut out);
        out
    }

    /// Decodes one slot, checksum and all.
    ///
    /// # Errors
    ///
    /// [`ManifestError::SlotTooShort`], [`ManifestError::NotAManifest`],
    /// [`ManifestError::UnknownVersion`], [`ManifestError::SlotChecksum`],
    /// [`ManifestError::StrideMismatch`] or [`ManifestError::UnknownVendor`] —
    /// checked in that order, so the most basic disagreement is the one
    /// reported.
    pub fn decode(slot: &[u8]) -> Result<Self, ManifestError> {
        Self::decode_with_layout(slot).map(|(header, _)| header)
    }

    /// [`ManifestHeader::decode`], keeping the geometry it resolved.
    ///
    /// Private, and it exists so nothing downstream has to resolve the version a
    /// second time and carry an `Err` arm for a version `decode` has already
    /// proved is known — an arm no input could reach is an arm no test can
    /// close.
    fn decode_with_layout(slot: &[u8]) -> Result<(Self, Layout), ManifestError> {
        let image = image_of(slot).map_err(|len| ManifestError::SlotTooShort { len })?;
        let magic: [u8; 8] = le_bytes(&image, H_MAGIC);
        if magic
            .iter()
            .take(MAGIC_FAMILY.len())
            .ne(MAGIC_FAMILY.iter())
        {
            return Err(ManifestError::NotAManifest);
        }
        let format_version = u16::from_le_bytes(le_bytes(&image, H_VERSION));
        // THE DISPATCH. Not a comparison against one constant: this build reads
        // every version in `Layout::KNOWN` and refuses the rest BY NUMBER, so a
        // version-1 census on disk keeps reading after version 2 is minted and a
        // version-3 file is refused rather than decoded at version 2's offsets.
        let layout = Layout::for_version(format_version)?;
        if magic != layout.magic() {
            // The magic and the version field disagree, and there is no way to
            // tell which of the two is the lie. Refused by version, because the
            // version field is the thing that selects the geometry.
            return Err(ManifestError::UnknownVersion(format_version));
        }
        verify(&image)
            .map_err(|(stored, computed)| ManifestError::SlotChecksum { stored, computed })?;
        let entry_stride = u16::from_le_bytes(le_bytes(&image, H_STRIDE));
        if u64::from(entry_stride) != layout.entry_stride() {
            return Err(ManifestError::StrideMismatch(entry_stride));
        }
        let name = text_at(&image, H_VENDOR, H_VENDOR_LEN).ok_or(ManifestError::UnknownVendor)?;
        let vendor = Vendor::ALL
            .into_iter()
            .find(|v| v.as_str() == name)
            .ok_or(ManifestError::UnknownVendor)?;
        Ok((
            Self {
                format_version,
                entry_stride,
                vendor,
                generation: u64::from_le_bytes(le_bytes(&image, H_GENERATION)),
                n_valid: u64::from_le_bytes(le_bytes(&image, H_N_VALID)),
                n_keys: u64::from_le_bytes(le_bytes(&image, H_N_KEYS)),
                total_rows: u64::from_le_bytes(le_bytes(&image, H_TOTAL_ROWS)),
            },
            layout,
        ))
    }

    /// Reads the whole header region and returns every generation that
    /// survived, newest first, together with anything stepped over on the way.
    ///
    /// Each of the first [`store::format::MAX_SLOTS`] slot positions is decoded
    /// independently, and every valid one whose generation the region's entry
    /// capacity supports becomes a candidate, in generation order. A slot that
    /// is empty, stale, half-written, corrupt or in the wrong position is not a
    /// candidate — and the reason it is not is **returned**, in
    /// [`HeaderRead::stepped_over`], rather than dropped.
    ///
    /// It does not condemn the file when the newest slot fails its check
    /// against the capacity. That is the crash where the header became durable
    /// before the entries it counts, and the previous generation is sitting
    /// intact in the other slot.
    ///
    /// # Errors
    ///
    /// [`ManifestError::HeaderRegionTooShort`],
    /// [`ManifestError::NoValidHeader`] when no slot decodes and none gave a
    /// specific reason, the most specific refusal any slot did give, or
    /// [`ManifestError::VendorMismatch`] when the surviving header names
    /// another vendor.
    pub fn read_region(
        region: &[u8],
        entries: &[u8],
        vendor: Vendor,
    ) -> Result<HeaderRead, ManifestError> {
        let slots = whole_slots(region);
        if slots < SLOT_COUNT {
            return Err(ManifestError::HeaderRegionTooShort {
                slots,
                need: SLOT_COUNT,
            });
        }

        let mut candidates: Vec<(Self, Layout)> = Vec::with_capacity(MAX_SLOTS);
        let mut fault: Option<ManifestError> = None;
        for (index, chunk) in (0u64..).zip(region.chunks(SLOT_STRIDE_LEN).take(MAX_SLOTS)) {
            match Self::decode_with_layout(chunk) {
                Ok((header, layout)) => {
                    let expected = header.generation % SLOT_COUNT;
                    if expected == index {
                        candidates.push((header, layout));
                    } else {
                        fault.get_or_insert(ManifestError::SlotPositionMismatch {
                            expected,
                            found: index,
                        });
                    }
                }
                Err(refusal) => {
                    if is_specific(refusal) {
                        fault.get_or_insert(refusal);
                    }
                }
            }
        }

        // Newest first. Bounded by the candidate list rather than by the
        // generation strictly decreasing: a loop that leaned on the comparison
        // would hang rather than fail if that comparison stopped being strict,
        // and a hang is the one failure a test suite cannot report.
        candidates.sort_unstable_by_key(|(header, _)| std::cmp::Reverse(header.generation));
        let mut newest: Option<(Self, Layout)> = None;
        let mut older: Option<(Self, Layout)> = None;
        // Kept apart from `fault` on purpose. A generation the region cannot
        // support is what an operator needs to hear about first — it is the
        // state a writer published — and it is reported for the NEWEST such
        // generation rather than the last one looked at.
        let mut unsupported: Option<ManifestError> = None;
        for (candidate, layout) in candidates {
            // THE CAPACITY IS THE CANDIDATE'S OWN, not the file's. Two slots may
            // name two versions — that is exactly the state a crash between the
            // upgrade's two writes leaves — and counting whole entries at one
            // stride to validate a counter written at the other accepts a header
            // claiming twice the entries the region holds.
            match candidate.validate(layout.capacity_for(entries)) {
                Ok(()) => {
                    if candidate.vendor != vendor {
                        return Err(ManifestError::VendorMismatch {
                            asked: vendor,
                            found: candidate.vendor,
                        });
                    }
                    // Two slots, so two places to put a survivor. The assertion
                    // below is what keeps that spelling honest: a third slot
                    // would silently overwrite `older` here, so a family that
                    // grew one would be a compile error rather than a lost
                    // generation.
                    if newest.is_none() {
                        newest = Some((candidate, layout));
                    } else {
                        older = Some((candidate, layout));
                    }
                }
                Err(refused) => {
                    unsupported.get_or_insert(refused);
                }
            }
        }
        let stepped_over = unsupported.or(fault);
        let Some((newest, newest_layout)) = newest else {
            return Err(stepped_over.unwrap_or(ManifestError::NoValidHeader));
        };
        Ok(HeaderRead {
            newest,
            newest_layout,
            older,
            stepped_over,
        })
    }
}

const _: () = assert!(MAX_SLOTS == 2);

/// What one header region held: the generations that survived, and what did
/// not.
///
/// Returned by [`ManifestHeader::read_region`] rather than a bare header,
/// because both of the other two answers are load-bearing and both used to be
/// thrown away. The older generation is what makes [`Manifest::load`]'s
/// fall-back real rather than only true for a region that is physically short,
/// and `stepped_over` is what makes the fall-back loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderRead {
    newest: ManifestHeader,
    newest_layout: Layout,
    older: Option<(ManifestHeader, Layout)>,
    stepped_over: Option<ManifestError>,
}

impl HeaderRead {
    /// The highest generation that survived. This is the committed state.
    #[must_use]
    pub const fn newest(&self) -> ManifestHeader {
        self.newest
    }

    /// The geometry [`HeaderRead::newest`] declared.
    ///
    /// Carried rather than re-resolved, so the walk that follows cannot resolve
    /// it differently from the decode that accepted it.
    #[must_use]
    pub const fn newest_layout(&self) -> Layout {
        self.newest_layout
    }

    /// The generation below it, if the other slot still holds one.
    ///
    /// The recovery point. A commit that became durable before the entries it
    /// counts leaves this one describing exactly the prefix that *is* durable.
    #[must_use]
    pub const fn older(&self) -> Option<ManifestHeader> {
        match self.older {
            Some((header, _)) => Some(header),
            None => None,
        }
    }

    /// The geometry [`HeaderRead::older`] declared.
    ///
    /// **The two slots may name two versions.** That is not hypothetical: it is
    /// exactly the state a crash between the upgrade's entry writes and its slot
    /// write leaves, and walking the older generation at the newer's stride
    /// would read its entries at offsets it never wrote.
    #[must_use]
    pub const fn older_layout(&self) -> Option<Layout> {
        match self.older {
            Some((_, layout)) => Some(layout),
            None => None,
        }
    }

    /// A refusal that was stepped over to produce [`HeaderRead::newest`].
    ///
    /// `Some` means the file is damaged even though a header was recovered: a
    /// slot that failed its checksum, one that names a version this build does
    /// not read, one found in the wrong position, or a generation whose counter
    /// the region cannot support. There is at most one, because a fault costs a
    /// slot and there are two.
    #[must_use]
    pub const fn stepped_over(&self) -> Option<ManifestError> {
        self.stepped_over
    }
}

/// One vendor's census, loaded.
///
/// Holds the header — the counters — and an index from key to the newest entry
/// for that key. The index is reserved from the **committed entry count**,
/// which is known before the walk begins and which `validate` has already
/// checked against the region, so the reservation is proportional to the census
/// rather than to the file it arrived in.
///
/// # Two paths, two different bounds, said separately
///
/// **Lookup — O(1) worst case.** [`Manifest::entry`] is one probe into a table
/// that the load walk built once and never grew, whatever the census holds.
/// `docs/07-o1-architecture.md` layer 3. C-12 in `crates/pull/benches/ratio.rs`
/// measures it at 1×, 10× and 100× the census.
///
/// **Append — O(1) worst case for the first `n_valid` new keys, amortised O(1)
/// after that.** The reservation carries [`APPEND_HEADROOM_FACTOR`]× the
/// census, so [`Manifest::record`] has at least `n_valid` free slots waiting
/// and cannot rehash until they are gone; past them one append in a doubling
/// rebuilds the table at `O(n_keys)`. Until D-0040 this sentence read "it never
/// rehashes … O(1) worst case", the reservation was exactly `n_valid`, and a
/// census sitting on a `7·2^k` boundary rehashed on the **first** append —
/// measured at 22× the cost between a 1,792-entry census and a 57,344-entry
/// one. `docs/06-limits.md` §23 states what is still not unconditional and what
/// removing the last arm would cost.
///
/// [`Manifest::reserved`] is the reservation, and M-17 and M-19 assert it.
///
/// # Why the log is held as well as the index
///
/// The entry region on disk **is** a log: `n_valid` entries in the order they
/// were committed, of which the index keeps only the newest per key. Holding
/// the index alone was enough while nothing could write, and it stopped being
/// enough the moment [`Manifest::image`] existed — a manifest that could not
/// reproduce the entries it was loaded from would rewrite one month's history
/// every time the file was replaced whole, silently, and `CLAUDE.md` §3 rule 8
/// does not admit that. It is also what makes the image *deterministic*: an
/// image emitted in `HashMap` iteration order would differ between two
/// processes given identical inputs, which is rule 5.
///
/// It costs `size_of::<Held>()` — **96 bytes** on this workspace's target,
/// pinned by `pull::unit::the_log_is_the_entry_region_in_order` — per committed
/// row, reserved through [`reservation_for`] exactly as the index is, so the
/// first `n_valid` appends after a load cannot reallocate it either. At the
/// measured 248,000-entry scale that is about 48 MB beside the index's 159 MB;
/// at [`MAX_ENTRIES`] it is 201 MB beside 637 MB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    header: ManifestHeader,
    log: Vec<Held>,
    index: HashMap<EntryKey, Held>,
    degraded: Option<ManifestError>,
    loaded_version: u16,
}

impl Manifest {
    /// An empty manifest for a vendor that has none yet.
    ///
    /// **Private, and D-0036 is the reason.** A writer that called this on a
    /// vendor which already had a manifest produced a census that was silently,
    /// verifiably wrong and still loaded clean: the stale slot 0 wins on
    /// generation, the fresh generation-1 commit lands in slot 1, entry 0 is
    /// overwritten, and — because real index months share a row count — the
    /// recomputed key count and row total still agree with the stale header. So
    /// the only public doors are [`Manifest::open`], which hands out a genesis
    /// only for a file that has nothing in it, and [`Manifest::load`].
    ///
    /// The index reserves nothing, because nothing is known yet to reserve
    /// from. That is a write-path cost — one rehash per doubling as keys are
    /// first recorded — and it is stated rather than hidden: layer 3's
    /// guarantee is about the loaded index, which is the one a query touches.
    /// See `docs/06-limits.md` §17.
    fn genesis(vendor: Vendor) -> Self {
        Self {
            header: ManifestHeader::genesis(vendor),
            log: Vec::new(),
            index: HashMap::new(),
            degraded: None,
            loaded_version: FORMAT_VERSION,
        }
    }

    /// The manifest of a vendor whose file may or may not exist yet.
    ///
    /// Both regions empty is a file with nothing in it, and that — and only
    /// that — is a genesis manifest. Anything else is [`Manifest::load`], so a
    /// writer cannot start a new census over a file that already holds one.
    ///
    /// # Errors
    ///
    /// Whatever [`Manifest::load`] refuses.
    pub fn open(
        vendor: Vendor,
        header_region: &[u8],
        entries: &[u8],
    ) -> Result<Self, ManifestError> {
        if header_region.is_empty() && entries.is_empty() {
            return Ok(Self::genesis(vendor));
        }
        Self::load(vendor, header_region, entries)
    }

    /// The manifest in one whole file's bytes, split at [`HEADER_LEN`].
    ///
    /// The inverse of [`Manifest::image`], and it exists rather than leaving
    /// the split to a caller because **the split point is part of the format**.
    /// A caller that computed it independently could be wrong about it in a way
    /// no test of this module would ever see, and the failure would be a census
    /// that reads its first entry out of the header region.
    ///
    /// # A first ingest is not a failure
    ///
    /// An empty slice is a file that does not exist yet, and
    /// [`Manifest::open`] answers it with a genesis census. That is the whole
    /// route to one: there is no `Manifest::empty`, and D-0036 is why —
    /// starting a fresh census over a file that already holds one produces a
    /// manifest that is verifiably wrong and still loads clean. Bytes that are
    /// present but shorter than [`HEADER_LEN`] are a partly-written file, and
    /// they are refused by name rather than quietly read as "nothing here yet".
    ///
    /// # Errors
    ///
    /// Whatever [`Manifest::open`] refuses —
    /// [`ManifestError::HeaderRegionTooShort`] for a file that is short,
    /// [`ManifestError::CounterExceedsRegion`] for one truncated below the
    /// count its header publishes.
    pub fn open_image(vendor: Vendor, file: &[u8]) -> Result<Self, ManifestError> {
        // `split_at_checked` rather than `split_at`: the panicking form would
        // be reached by any file shorter than the header region, which is the
        // ordinary shape of an interrupted write rather than a rare one. The
        // `None` arm is that file, and it is handed on whole so `open` refuses
        // it for its length.
        let (header_region, entries) = file
            .split_at_checked(HEADER_REGION_LEN)
            .unwrap_or((file, &[]));
        Self::open(vendor, header_region, entries)
    }

    /// Loads a manifest from its header region and its entry region.
    ///
    /// This is the one scan, and it is where the counters are earned: every
    /// entry below `n_valid` is decoded and checksum-verified, the distinct
    /// keys and the row total are recomputed, and a header whose counters do
    /// not match what the entries actually hold is refused. Afterwards every
    /// question this type answers is a field read or one hash probe.
    ///
    /// # It walks *down* the generations, and says that it did
    ///
    /// If the newest committed generation does not describe what is on disk —
    /// its tail entry is zeroed, or garbage, or half-written, all of which are
    /// ordinary results of a crash between a data write and its flush — the
    /// generation below it is tried, because that one counts a prefix of these
    /// same entries and the whole point of alternating slots is that it is
    /// still intact. Whichever generation loads,
    /// [`Manifest::degraded_reason`] carries what was stepped over. Before
    /// D-0036 the fall-back existed only for a region that was physically too
    /// **short**, so the commonest shape of the crash it was written for
    /// condemned the file and left recovery to the ~248,000-file directory walk
    /// this module exists to avoid.
    ///
    /// # Errors
    ///
    /// [`ManifestError`] — a header that does not decode, a counter the region
    /// cannot support, an entry that fails its own checksum or its own checks,
    /// a key whose history goes backwards, or a counter that disagrees with the
    /// entries it claims to count. When no generation survives, the refusal
    /// reported is the **newest** one's: that is the state a writer published,
    /// and `ManifestError` is `Copy` and does not nest.
    pub fn load(
        vendor: Vendor,
        header_region: &[u8],
        entries: &[u8],
    ) -> Result<Self, ManifestError> {
        let read = ManifestHeader::read_region(header_region, entries, vendor)?;

        let header = read.newest();
        let published = match Self::walk(header, read.newest_layout(), entries) {
            Ok(census) => {
                return Ok(Self {
                    header,
                    log: census.log,
                    index: census.index,
                    degraded: read.stepped_over(),
                    loaded_version: header.format_version,
                });
            }
            Err(fault) => fault,
        };
        // The published generation does not describe these bytes. The one below
        // it counts a prefix of them, and `read_region` has already validated
        // it against the region. Nothing is lost by not also carrying
        // `stepped_over` here: a stepped-over slot costs a candidate, and with
        // two slots a file cannot both have one and still offer an older
        // generation.
        let (Some(previous), Some(previous_layout)) = (read.older(), read.older_layout()) else {
            return Err(published);
        };
        Self::walk(previous, previous_layout, entries)
            .map(|census| Self {
                header: previous,
                log: census.log,
                index: census.index,
                degraded: Some(published),
                loaded_version: previous.format_version,
            })
            .map_err(|_| published)
    }

    /// Decodes every committed entry under one header and checks the counters
    /// against them.
    ///
    /// Split out of [`Manifest::load`] so that walking a second generation is
    /// the same code rather than a paraphrase of it.
    fn walk(
        header: ManifestHeader,
        layout: Layout,
        entries: &[u8],
    ) -> Result<Census, ManifestError> {
        // Reserved from the COUNTER, which `validate` has already proved is
        // within both `MAX_ENTRIES` and the region's capacity -- not from the
        // region's byte length, which is untrusted input. Sizing it from the
        // length made a one-entry census inside a region at the design ceiling
        // allocate 574 MB, because the reservation followed the file rather
        // than the census.
        //
        // The census is then multiplied by `APPEND_HEADROOM_FACTOR` and capped
        // at the design ceiling, so the walk cannot rehash AND the appends
        // after it cannot either, for as many new keys as the census already
        // holds. Reserving exactly `n_valid` left zero free slots at every
        // count of the form 7·2^k and turned the next append into an O(keys)
        // rebuild; the numbers that measurement produced are in
        // `APPEND_HEADROOM_FACTOR`'s note, and so is the derivation of the two.
        //
        // The log is reserved from the same number for the same reason: a push
        // onto a `Vec` with no spare capacity copies the whole log, so a
        // reservation of exactly `n_valid` would move the O(census) arm out of
        // the map and into the log rather than removing it.
        let mut index: HashMap<EntryKey, Held> =
            HashMap::with_capacity(reservation_for(header.n_valid));
        let mut log: Vec<Held> = Vec::with_capacity(reservation_for(header.n_valid));
        let mut n_keys = 0u64;

        // CHUNKED AT THE HEADER'S OWN STRIDE, from the layout its version
        // selected. Reading a version-1 region in 128-byte steps would decode
        // every second entry as the tail of the one before it.
        for (ordinal, chunk) in
            (0u64..header.n_valid).zip(entries.chunks_exact(layout.entry_stride_len()))
        {
            let held = layout
                .decode_entry(chunk)
                .map_err(|fault| ManifestError::Entry { ordinal, fault })?;
            let entry = held.entry;
            log.push(held);
            match index.insert(entry.key, held) {
                None => n_keys += 1,
                Some(previous) => {
                    if entry.rows < previous.entry.rows {
                        return Err(ManifestError::RowCountWentBackwards {
                            ordinal,
                            previous: previous.entry.rows,
                            next: entry.rows,
                        });
                    }
                    if entry.last_ts_micros < previous.entry.last_ts_micros {
                        return Err(ManifestError::KeyTimestampsOutOfOrder {
                            ordinal,
                            previous: previous.entry.last_ts_micros,
                            next: entry.last_ts_micros,
                        });
                    }
                }
            }
        }

        let mut total_rows = 0u64;
        for held in index.values() {
            total_rows = total_rows
                .checked_add(held.entry.rows)
                .ok_or(ManifestError::RowTotalOverflow)?;
        }
        if n_keys != header.n_keys {
            return Err(ManifestError::KeyCountDisagrees {
                header: header.n_keys,
                entries: n_keys,
            });
        }
        if total_rows != header.total_rows {
            return Err(ManifestError::RowTotalDisagrees {
                header: header.total_rows,
                entries: total_rows,
            });
        }
        Ok(Census { log, index })
    }

    /// The header, counters and all.
    #[must_use]
    pub const fn header(&self) -> ManifestHeader {
        self.header
    }

    /// What this census had to step over to load, if anything.
    ///
    /// `Some` means the file is damaged and this manifest is what could still
    /// be recovered from it: a header slot that failed its checksum, a
    /// generation whose entries were not all there, a slot in the wrong
    /// position. The counters below are then the *recovered* generation's, and
    /// a month that a later generation had committed is not in them.
    ///
    /// **An operator entry point renders this.** `CLAUDE.md` §4 allows
    /// degrading loudly and naming the reason, or refusing — and this crate
    /// performs no I/O and holds no logger, so the loudest thing available to
    /// it is a value the census carries and a `#[must_use]` on the way to it.
    #[must_use]
    pub const fn degraded_reason(&self) -> Option<ManifestError> {
        self.degraded
    }

    /// How many entries the index reserved room for — layer 3, as a number.
    ///
    /// `docs/07-o1-architecture.md`: *"Layer 3's guarantee is the absence of a
    /// rehash, so the bound is the reservation itself."* A guarantee about an
    /// allocation that nothing can observe is a guarantee nothing can test, so
    /// this exposes it, and two tests read it for two different reasons:
    ///
    /// * `pull::unit::the_loaded_index_is_reserved_from_the_census` — the
    ///   reservation follows the census and **not** the region it arrived in.
    /// * `pull::unit::a_loaded_index_carries_headroom_for_the_appends_after_it`
    ///   — it is at least [`APPEND_HEADROOM_FACTOR`] × the census (or the
    ///   design ceiling, whichever is smaller), which is what makes the free
    ///   slots [`Manifest::record`] relies on a checked number rather than
    ///   whatever `HashMap`'s rounding happened to leave. That rounding leaves
    ///   **nothing** at every census of the form `7·2^k`, which is how the
    ///   append path came to be `O(n_keys)` behind an O(1) comment. D-0040.
    ///
    /// This is the table's capacity, not its length: it is always at least
    /// [`Manifest::keys`] and normally larger.
    #[must_use]
    pub fn reserved(&self) -> usize {
        self.index.capacity()
    }

    /// How many entries have been committed. One field read.
    #[must_use]
    pub const fn entries(&self) -> u64 {
        self.header.n_valid
    }

    /// How many distinct `(instrument, timeframe, month)` keys are held. One
    /// field read — the answer to "how many month files do I have", without a
    /// directory walk.
    #[must_use]
    pub const fn keys(&self) -> u64 {
        self.header.n_keys
    }

    /// How many rows are held across every key. One field read.
    #[must_use]
    pub const fn total_rows(&self) -> u64 {
        self.header.total_rows
    }

    /// The version this census was **loaded** from.
    ///
    /// [`FORMAT_VERSION`] for a genesis census and for one read from a file this
    /// build wrote. Older for a file written before the version it names was
    /// minted — and that number does not move when [`Manifest::record`] is
    /// called, because it is a fact about the bytes on disk, not about the
    /// bytes this census will publish.
    #[must_use]
    pub const fn loaded_version(&self) -> u16 {
        self.loaded_version
    }

    /// Whether publishing this census rewrites it at a newer version.
    ///
    /// **A writer must not append positionally when this is true.** The
    /// incremental path writes one entry at `HEADER_LEN + ordinal·stride` at the
    /// stride this build *writes*, and the file on disk is at the stride it was
    /// *written* at; against a version-1 file those two disagree from the first
    /// entry onward. `pull::ingest::install_census` installs the whole image
    /// instead, which is the same thing it already does for a repair and for a
    /// file that does not exist yet.
    ///
    /// The upgrade is therefore paid **once, and only by a run that had
    /// something to record**. A run that changes nothing installs nothing and
    /// leaves the version-1 file byte for byte as it was — `CLAUDE.md` §3 rule
    /// 5 is about the bytes.
    #[must_use]
    pub const fn upgrading(&self) -> bool {
        self.loaded_version != FORMAT_VERSION
    }

    /// The newest entry for one key, if any. One hash probe.
    #[must_use]
    pub fn entry(&self, key: &EntryKey) -> Option<Entry> {
        self.index.get(key).map(|held| held.entry)
    }

    /// The newest row for one key — the entry and its closes. One hash probe.
    #[must_use]
    pub fn held(&self, key: &EntryKey) -> Option<Held> {
        self.index.get(key).copied()
    }

    /// The month's closes, if the census holds that month. **One hash probe.**
    ///
    /// `None` means the month is not held. [`Closes::UNKNOWN`] means it is held
    /// and no close was ever recorded for it — a version-1 entry, or one this
    /// build wrote without reading the bar file. Those are two different facts
    /// and this is where they stop being one.
    ///
    /// # Cost, and what crossing a month costs
    ///
    /// **O(1) worst case**, the same bound [`Manifest::entry`] carries: one
    /// probe into a table the load walk built once and never grew. A caller
    /// computing a month-over-month change probes twice — this month and
    /// `YearMonth::previous` — which crosses a manifest **entry**, not a file:
    /// no `open`, no `stat`, no `pread`, because the whole census is already
    /// resident. Two probes, zero syscalls.
    ///
    /// That bound is **measured, not asserted**: `C-12` —
    /// `pull::bench::entry_lookup_is_flat` — times this same `self.index`
    /// probe at 1×, 10× and 100× the census, on a hit and on a miss. The bench
    /// calls [`Manifest::entry`] rather than this method because the two are
    /// one `HashMap::get` on the same key type and differ only in which field
    /// of the `Copy` value they hand back, so a cost that had started to grow
    /// with the census would show on either one.
    #[must_use]
    pub fn closes(&self, key: &EntryKey) -> Option<Closes> {
        self.index.get(key).map(|held| held.closes)
    }

    /// Every key this census holds, in no particular order.
    ///
    /// # Why a census needs to be readable and not only probeable
    ///
    /// [`Self::entry`] answers *is this key held*, which is the right question
    /// when the caller already knows which key to ask about. `crates/api`'s
    /// coverage grid did not: it built its own keys out of the instrument
    /// master and probed those, and **every probe missed**. The store held 194
    /// months of expired futures under the names the vendor archive used —
    /// `ABB-III`, segment `FNO` — and the grid was asking about `NIFTY` in
    /// segment `INDEX`, a key nothing had ever written. The page therefore
    /// reported `0 of 200 held` on the same screen as `62,978 rows`, which is
    /// two contradictory claims about one file and exactly the silent shape
    /// `CLAUDE.md` §4 bans.
    ///
    /// A census that cannot be enumerated can only confirm a guess. This is how
    /// a caller asks *what is here* instead.
    ///
    /// # Cost, stated
    ///
    /// **O(keys), and it allocates nothing.** That makes it the one operation on
    /// this type that is not O(1), so it belongs where `Manifest::open` already
    /// is — read once, at startup — and not inside a request. `crates/api`'s
    /// `Site` holds the sorted result for the process's lifetime, which is the
    /// same bargain D-0039 struck for the censuses themselves.
    /// `docs/06-limits.md` §32 records it.
    ///
    /// The order is a `HashMap`'s and therefore **not stable between runs**. A
    /// caller that renders these must sort them, or it will address a different
    /// row on every reload — see `api::census::held_series`.
    pub fn held_keys(&self) -> impl Iterator<Item = &EntryKey> {
        self.index.keys()
    }

    /// Records one month, returning the two writes that publish it.
    ///
    /// The manifest is append-only: an update to a key that already exists is a
    /// **new entry**, and the newest wins. Nothing on disk is ever rewritten in
    /// place, so a crash during an update leaves the previous state whole.
    ///
    /// `self` is left untouched unless every check passes, so a refused record
    /// cannot leave the in-memory counters describing a write that never
    /// happened.
    ///
    /// A census that loaded degraded may still be appended to, and that is
    /// deliberate: recovering from a torn commit *is* writing the next
    /// generation, and refusing here would leave the commonest crash with no
    /// way forward at all. What the writer may not do is append without having
    /// looked at [`Manifest::degraded_reason`], which is why that answer is
    /// carried on the value rather than logged and forgotten.
    ///
    /// # What one call costs, stated exactly
    ///
    /// Every step here is fixed work — one probe, four counter updates, one
    /// 64-byte entry image, one 64-byte slot image — **except** the
    /// `index.insert` that keeps the census current, and that one has a
    /// condition on it:
    ///
    /// * **O(1) worst case for the first `n_valid` calls after a load.**
    ///   `Manifest::walk` reserved [`APPEND_HEADROOM_FACTOR`]× the census and
    ///   the walk can leave at most `n_valid` elements in it, so at least
    ///   `n_valid` slots are free and no call in that run can rebuild the
    ///   table.
    /// * **Amortised O(1) after that**, with an `O(n_keys)` worst case: one
    ///   call in a doubling rebuilds the table and the rest are O(1). Named as
    ///   amortised rather than dressed as worst case.
    ///
    /// **An update to a key already held is not exempt from the second bullet,
    /// and it is easy to assume it is.** It consumes no slot — the key count
    /// does not move — but `HashMap::insert` asks the table for one slot
    /// *before* it looks the key up, so on a table with none left it grows
    /// anyway. That was written here as "never grows" until the test below
    /// refused it: `pull::unit::a_loaded_index_carries_headroom_for_the_appends_after_it`
    /// asserts both halves, the update inside the headroom that does hold and
    /// the one past it that does not.
    ///
    /// A manifest opened at genesis reserves nothing, because there is no
    /// census to size a reservation from, so its write path is amortised O(1)
    /// from the first key. `docs/06-limits.md` §17 and §23.
    ///
    /// # Errors
    ///
    /// [`ManifestError::Entry`] for an entry that is not internally consistent,
    /// [`ManifestError::RowCountWentBackwards`] or
    /// [`ManifestError::KeyTimestampsOutOfOrder`] for one that contradicts what
    /// is already recorded for its key, [`ManifestError::RowTotalOverflow`], or
    /// whatever [`ManifestHeader::advance`] and [`ManifestHeader::commit`]
    /// refuse.
    pub fn record(&mut self, entry: Entry) -> Result<Append, ManifestError> {
        self.record_held(Held::unknown(entry))
    }

    /// Records one month **with the closes read off its bar file**.
    ///
    /// [`Manifest::record`] is this with [`Closes::UNKNOWN`], for a caller that
    /// has a counter row and no price. Everything else about the two is
    /// identical, including every refusal.
    ///
    /// # Errors
    ///
    /// The same list [`Manifest::record`] carries.
    pub fn record_held(&mut self, held: Held) -> Result<Append, ManifestError> {
        let entry = held.entry;
        let ordinal = self.header.n_valid;
        entry
            .check()
            .map_err(|fault| ManifestError::Entry { ordinal, fault })?;

        let (n_keys, total_rows) = match self.index.get(&entry.key) {
            None => (
                self.header.n_keys + 1,
                self.header
                    .total_rows
                    .checked_add(entry.rows)
                    .ok_or(ManifestError::RowTotalOverflow)?,
            ),
            Some(previous) => {
                if entry.rows < previous.entry.rows {
                    return Err(ManifestError::RowCountWentBackwards {
                        ordinal,
                        previous: previous.entry.rows,
                        next: entry.rows,
                    });
                }
                if entry.last_ts_micros < previous.entry.last_ts_micros {
                    return Err(ManifestError::KeyTimestampsOutOfOrder {
                        ordinal,
                        previous: previous.entry.last_ts_micros,
                        next: entry.last_ts_micros,
                    });
                }
                // Subtraction without a check: the comparison two lines above
                // returned early unless `entry.rows >= previous.rows`, and
                // `previous.rows` was itself added into the total. A
                // `checked_sub` here would carry a failure arm no input could
                // reach, which is a branch nobody has checked.
                (
                    self.header.n_keys,
                    self.header
                        .total_rows
                        .checked_add(entry.rows - previous.entry.rows)
                        .ok_or(ManifestError::RowTotalOverflow)?,
                )
            }
        };

        // `advance` has refused a counter past `MAX_ENTRIES`, so `ordinal` is
        // below it and both offsets below are plain arithmetic. Calling the
        // fallible entry points here would add two `?` arms that no input could
        // ever take — a branch nobody has checked, sitting behind a coverage
        // gate that would still report 100%.
        //
        // `advance` also restates the version as the one this build writes, so
        // the geometry below is `Layout::CURRENT` on every path, including the
        // one that loaded a version-1 file.
        let header = self.header.advance(1, n_keys, total_rows)?;
        let offset = Layout::CURRENT.offset_within_bounds(ordinal);
        let commit = header.commit_at(Layout::CURRENT);

        self.log.push(held);
        self.index.insert(entry.key, held);
        self.header = header;
        Ok(Append {
            ordinal,
            offset,
            bytes: held.image(),
            commit,
        })
    }

    /// The complete file this census is: the header region, then every
    /// committed entry in the order it was recorded.
    ///
    /// The inverse of [`Manifest::open_image`]. These are the bytes
    /// [`ManifestHeader::read_region`] and [`Manifest::load`] already read —
    /// **nothing in the reader was relaxed to accept them**, which is the point
    /// of the round trip that proves it. Slot `generation % SLOT_COUNT` carries
    /// the header and the other slot is left zero: zeroes decode as
    /// [`ManifestError::NotAManifest`], which `read_region` does not count as a
    /// fault, so a freshly imaged file loads with
    /// [`Manifest::degraded_reason`] `None`.
    ///
    /// # It is O(entries), and it is not the counter this layer is about
    ///
    /// Every entry is re-imaged and re-checksummed, so one call is
    /// `O(n_valid)`. That is the write path, where the bytes have to be
    /// produced whatever their order; the census itself is still a field read.
    /// The **incremental** writer remains [`Manifest::record`] — one 64-byte
    /// entry and one 64-byte commit, however large the census — and this is for
    /// the file that does not exist yet and for one being replaced whole.
    ///
    /// # Atomicity: the buffer is one unit, the install is the caller's
    ///
    /// **The image is atomic, its installation is not, and a half-installed
    /// image is refused rather than believed.** This crate performs no I/O and
    /// cannot rename anything, so that is the honest statement rather than a
    /// guarantee it is in no position to make.
    ///
    /// One `Vec<u8>` reaches a writer whole or not at all, so there is no API
    /// here that can emit half a census. Installing it must be write to a
    /// temporary, flush, then rename onto the live path: a rename within one
    /// filesystem is the only step atomic against a crash, and overwriting the
    /// live file in place is not — it publishes a prefix.
    ///
    /// A prefix that does reach disk is **detected**, which is what makes the
    /// gap survivable instead of silent. The header region is at the front, so
    /// a truncated install publishes a counter over entries that are not there
    /// and [`ManifestHeader::validate`] refuses it with
    /// [`ManifestError::CounterExceedsRegion`]; a torn slot fails its own
    /// CRC-32C and a torn entry fails its own.
    ///
    /// # What round-trips, exactly
    ///
    /// [`Manifest::open_image`] of this returns a value equal to `self` — every
    /// counter, the generation, every entry in order — for a manifest whose
    /// [`Manifest::degraded_reason`] is `None`. For one that loaded degraded
    /// the image is the **repair**: the recovered generation is written whole
    /// into a clean file, and what loads back differs in exactly that one
    /// field, because the damage it named is gone.
    ///
    /// # Panics
    ///
    /// It does not. `n_valid` is at most [`MAX_ENTRIES`] on every path that can
    /// produce a `Manifest` — [`ManifestHeader::validate`] on load,
    /// [`ManifestHeader::advance`] on record — so the slot image is built by
    /// the same infallible arithmetic [`Manifest::record`] uses, and there is
    /// no failure arm here for a test to be unable to reach.
    #[must_use]
    pub fn image(&self) -> Vec<u8> {
        // AT THE VERSION THIS BUILD WRITES, always. A census loaded from a
        // version-1 file images as version 2, with every entry it read carrying
        // `Closes::UNKNOWN` — which is the truth about them: version 1 had
        // nowhere to record a close, so nothing ever did.
        let commit = self.header.at_current().commit_at(Layout::CURRENT);
        // `log.len()` is at most `MAX_ENTRIES`, so this product is at most
        // 268,435,456 and the sum cannot overflow a `usize` on any target that
        // could hold the log in the first place.
        let mut out = Vec::with_capacity(HEADER_REGION_LEN + self.log.len() * ENTRY_LEN);

        // Two slots, written out longhand rather than indexed into, because
        // this workspace denies slice indexing and the family is two: the
        // assertion beside `MAX_SLOTS` is what makes a third one a compile
        // error rather than a slot this loop silently skips.
        if commit.slot == 0 {
            out.extend_from_slice(&commit.bytes);
        }
        out.resize(SLOT_STRIDE_LEN, 0);
        if commit.slot != 0 {
            out.extend_from_slice(&commit.bytes);
        }
        out.resize(HEADER_REGION_LEN, 0);

        for held in &self.log {
            out.extend_from_slice(&held.image());
        }
        out
    }

    /// The byte offset of entry `ordinal` **at the version this build writes**.
    ///
    /// `HEADER_LEN + ordinal·128`. An add and a multiply — law 4. There is no
    /// index to consult and nothing to search.
    ///
    /// This is [`Manifest::record`]'s geometry, which is the one a writer needs,
    /// because this build writes exactly one version. [`Layout::offset_of`] is
    /// the door that **dispatches**, and it is the one a reader addressing a
    /// file of unknown version must use.
    ///
    /// # Errors
    ///
    /// [`ManifestError::OrdinalOutOfRange`] past [`MAX_ENTRIES`]. The bound is
    /// on the ordinal rather than on the product, so the arithmetic itself is
    /// total: `MAX_ENTRIES · 128 + HEADER_LEN` is 268 MB and nowhere near `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::manifest::{ENTRY_STRIDE, Manifest, ManifestError, HEADER_LEN, MAX_ENTRIES};
    /// assert_eq!(Manifest::offset_of(0)?, HEADER_LEN);
    /// assert_eq!(Manifest::offset_of(1)?, HEADER_LEN + ENTRY_STRIDE);
    /// assert_eq!(Manifest::offset_of(1_000)?, HEADER_LEN + 128_000);
    /// assert!(Manifest::offset_of(MAX_ENTRIES + 1).is_err());
    /// # Ok::<(), ManifestError>(())
    /// ```
    pub const fn offset_of(ordinal: u64) -> Result<u64, ManifestError> {
        Layout::CURRENT.offset_of(ordinal)
    }
}

/// What one walk of the entry region produced.
///
/// Two structures over the same entries, and neither is derivable from the
/// other: the log is what the region holds, in order, and the index is the
/// newest entry per key. Returned together from one walk because building them
/// in two passes would decode and checksum every entry twice.
struct Census {
    /// Every committed row, in the order the region holds them.
    log: Vec<Held>,
    /// The newest row for each distinct key.
    index: HashMap<EntryKey, Held>,
}

const _: () = assert!(MAX_ENTRIES * ENTRY_STRIDE + HEADER_LEN < u64::MAX);

/// The eight magic bytes for a version, or the family alone for one no
/// [`Layout`] declares.
///
/// The second arm is reachable — [`ManifestHeader`]'s fields are public, so a
/// header naming version 7 can be imaged — and what it writes matters: leaving
/// the family bytes in place means [`ManifestHeader::decode`] refuses the slot
/// by **version**, which names the actual problem, rather than as "not a
/// manifest", which names the wrong one.
fn magic_for(version: u16) -> [u8; 8] {
    Layout::for_version(version).map_or_else(
        |_| {
            let mut out = [0u8; 8];
            for (dst, src) in out.iter_mut().zip(MAGIC_FAMILY.iter()) {
                *dst = *src;
            }
            out
        },
        Layout::magic,
    )
}

/// Whether a refusal says something about the file, or only that a slot is not
/// a header at all.
const fn is_specific(refusal: ManifestError) -> bool {
    !matches!(
        refusal,
        ManifestError::NotAManifest | ManifestError::SlotTooShort { .. }
    )
}

/// How many whole slot positions the region holds, capped at the family bound.
fn whole_slots(region: &[u8]) -> u64 {
    region
        .chunks_exact(SLOT_STRIDE_LEN)
        .take(MAX_SLOTS)
        .fold(0u64, |seen, _| seen + 1)
}

/// Copies the first [`IMAGE_LEN`] bytes, or reports how few there were.
///
/// One observation of the caller's bytes. Everything downstream reads this
/// array and nothing touches the caller's slice again, so the checksum covers
/// exactly the bytes the decoded value is built from.
fn image_of(bytes: &[u8]) -> Result<[u8; IMAGE_LEN], usize> {
    image_of_at(bytes, 0)
}

/// Copies the [`IMAGE_LEN`] bytes beginning at `start`, or reports how few
/// there were **in total**.
///
/// The length reported is the caller's whole slice rather than what was left
/// past `start`, because that is the number an operator can act on: an entry
/// region truncated mid-row is short by however much it is short of a whole
/// entry, not of a half.
fn image_of_at(bytes: &[u8], start: usize) -> Result<[u8; IMAGE_LEN], usize> {
    if bytes.len() < start + IMAGE_LEN {
        return Err(bytes.len());
    }
    let mut image = [0u8; IMAGE_LEN];
    for (dst, src) in image.iter_mut().zip(bytes.iter().skip(start)) {
        *dst = *src;
    }
    Ok(image)
}

/// Writes the checksum over bytes `0..60` into bytes `60..64`.
fn seal(image: &mut [u8; IMAGE_LEN]) {
    let crc = crc32c(&covered(image));
    write_at(image, OFF_CRC, crc.to_le_bytes());
}

/// Checks an image's stored checksum, reporting both numbers when it fails.
fn verify(image: &[u8; IMAGE_LEN]) -> Result<(), (u32, u32)> {
    let stored = u32::from_le_bytes(le_bytes(image, OFF_CRC));
    let computed = crc32c(&covered(image));
    if stored == computed {
        Ok(())
    } else {
        Err((stored, computed))
    }
}

/// Every byte an image's checksum covers: `0..60`, one contiguous run.
///
/// **The domain is frozen.** Covering the four checksum bytes as well would be
/// a different number over a different domain, and every image already on disk
/// would fail its check — a change no round-trip test would notice, because a
/// checksum that agrees with itself always agrees with itself.
/// `pull::unit::the_covered_domain_is_the_image_minus_its_checksum` pins it.
///
/// Returned as an owned array rather than as a sub-slice because this workspace
/// denies slice indexing. Sixty bytes copied once per entry is not on a path
/// that repeats: the census is answered from the header.
fn covered(image: &[u8; IMAGE_LEN]) -> [u8; OFF_CRC] {
    let mut head = [0u8; OFF_CRC];
    for (dst, src) in head.iter_mut().zip(image.iter()) {
        *dst = *src;
    }
    head
}

/// Writes `src` at `offset`.
fn write_at<const N: usize>(out: &mut [u8; IMAGE_LEN], offset: usize, src: [u8; N]) {
    for (dst, byte) in out.iter_mut().skip(offset).zip(src) {
        *dst = byte;
    }
}

/// Writes `text` at `offset`, leaving the rest of its field zero.
fn write_text(out: &mut [u8; IMAGE_LEN], offset: usize, text: &str) {
    for (dst, byte) in out.iter_mut().skip(offset).zip(text.bytes()) {
        *dst = byte;
    }
}

/// Reads `N` little-endian bytes at `offset`.
///
/// Index-free by construction — this workspace denies slice indexing, and a
/// decoder is exactly the place a panicking index would eventually be reached
/// by a corrupt file. Every caller has already copied a whole [`IMAGE_LEN`]
/// image, so no read is ever short.
fn le_bytes<const N: usize>(image: &[u8; IMAGE_LEN], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    for (dst, src) in out.iter_mut().zip(image.iter().skip(offset)) {
        *dst = *src;
    }
    out
}

/// The NUL-terminated text in a fixed-width field, or `None` if it is not
/// UTF-8.
fn text_at(image: &[u8; IMAGE_LEN], start: usize, len: usize) -> Option<&str> {
    let field = image.get(start..start + len).unwrap_or(&[]);
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    std::str::from_utf8(field.get(..end).unwrap_or(&[])).ok()
}
