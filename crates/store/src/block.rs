//! The per-block checksum: what turns a lost write into a refusal.
//!
//! # Why this module exists
//!
//! [`crate::format::FLAG_CHECKSUMS`] was declared and never implemented, which
//! left a real hole rather than a missing feature. The header commit's whole
//! crash argument assumes the appended records reach stable storage before the
//! slot write. When that assumption is broken — and it is the *likely* break,
//! because page 0 is re-dirtied on every commit and is therefore the hottest
//! writeback candidate in the file — the header names records whose bytes are
//! zeros from a newly allocated extent.
//!
//! Nothing in the record can detect that. An all-zero [`crate::format::Bar`]
//! satisfies `ohlc_is_sane`, and its `open_interest` of zero is a **real**
//! zero rather than [`crate::format::OI_NULL`], so a spot-index file quietly
//! acquires derivative-shaped bars that go on to enter a sweep. A checksum
//! taken when the records were written is the only thing that can say those
//! bytes are not the bytes that were committed.
//!
//! # The domain is the committed prefix, not the nominal block
//!
//! [`Layout::covered_byte_range`] is the range, and both sides derive it from
//! the same `n_valid`. The tail block of a file holds only the records the
//! counter covers, so checksumming its nominal 4088 bytes would read past EOF
//! for 72 of every 73 states a file passes through. A writer recomputes the
//! tail block's checksum on every commit that lands in it; a reader verifies
//! against the `n_valid` it read from the header. Same counter, same length,
//! same answer — `CLAUDE.md` §3 rule 5.
//!
//! One exception to "same counter", and it is a crash: the writer seals the
//! sidecar before it writes the header slot, so an append that dies between
//! the two leaves the tail block's entry sealed over MORE records than the
//! header claims. [`verify_through`] admits that block only when the stored
//! number is proved to be the checksum of the committed bytes followed by
//! records the file really holds, and says so on the log. Any other mismatch
//! is still refused. D-0688.
//!
//! # No I/O
//!
//! This module takes bytes and returns numbers. The sidecar file it feeds is
//! [`crate::path::FileKind::Checksums`]; reading and writing it belongs to a
//! writer that does not exist yet.

use crate::crc::crc32c;
use crate::format::FormatError;
use crate::header::Header;
use crate::layout::Layout;

/// The checksum a writer stores for `block`, given that block's bytes.
///
/// `bytes` must be exactly the range [`Layout::covered_byte_range`] names for
/// this block under this counter — offering more or fewer is refused rather
/// than trimmed, because a checksum over the wrong length is a number that
/// will never match again and nobody will know why.
///
/// # Errors
///
/// [`FormatError::BlockNotCommitted`] for a block past the counter,
/// [`FormatError::BlockLengthMismatch`] when `bytes` is not the covered
/// length, or [`FormatError::OffsetOverflow`] from the geometry.
///
/// # Examples
///
/// ```
/// # use store::{block, format::FormatError, layout::Layout};
/// let v2 = Layout::V2;
/// // One record committed: the tail block covers exactly that record.
/// let record = [7u8; 56];
/// let sealed = block::seal(v2, 1, 0, &record)?;
/// assert_eq!(block::seal(v2, 1, 0, &record)?, sealed, "idempotent");
/// // The same bytes at a length the geometry does not give the block.
/// assert!(block::seal(v2, 1, 0, &[7u8; 55]).is_err());
/// # Ok::<(), FormatError>(())
/// ```
pub fn seal(layout: Layout, n_valid: u64, block: u64, bytes: &[u8]) -> Result<u32, FormatError> {
    let need = covered_len(layout, n_valid, block)?;
    let len = byte_count(bytes);
    if len != need {
        return Err(FormatError::BlockLengthMismatch {
            block,
            len: bytes.len(),
            need,
        });
    }
    Ok(crc32c(bytes))
}

/// A verification asked of a file that carries no checksums, on the rolling
/// log.
///
/// # This path cannot be reached in production today, and that is the point
///
/// No writer in this workspace sets [`crate::format::FLAG_CHECKSUMS`] —
/// `docs/04-invariants.md` S-06 and S-06b record exactly that, and [`seal`] and
/// [`verify`] have no production caller because of it. So the operator who
/// first trips this arm is the operator running the *first* build that turns
/// checksums on, against files written by every build before it. Their file is
/// not corrupt; it simply predates the flag. Without this line the only record
/// of that is a `ChecksumsAbsent` returned to a caller that may well treat it
/// as a verification failure, which is the conflation S-06b exists to forbid:
/// "verified" and "there was nothing to verify against" are different answers.
///
/// `Warn`, because no byte is wrong — and one event per block *asked for*,
/// never per record.
fn note_unverifiable(header: &Header, block: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.block", "no checksums to verify against")
            .with("block", telemetry::Value::Uint(block))
            .with("n_valid", telemetry::Value::Uint(header.n_valid))
            .with("flags", telemetry::Value::Uint(u64::from(header.flags)))
            .with(
                "symbol_id",
                telemetry::Value::Uint(u64::from(header.symbol_id)),
            ),
    );
}

/// A block whose committed bytes are not the bytes that were sealed.
///
/// # What nothing else in this workspace can say
///
/// This is the lost write the module header describes. An all-zero
/// [`crate::format::Bar`] satisfies `ohlc_is_sane`, and its `open_interest` of
/// zero is a **real** zero rather than [`crate::format::OI_NULL`], so a
/// spot-index file quietly acquires derivative-shaped bars that go on to enter
/// a sweep. No range check, no header field and no downstream reader can tell
/// that extent from data. The checksum can — and until now it could tell only
/// its immediate caller, in a return value that a batch verifier would fold
/// into a count.
///
/// # Unreachable in production today, said plainly rather than left to be found
///
/// Same reason as [`note_unverifiable`]: nothing sets
/// [`crate::format::FLAG_CHECKSUMS`], so `verify` has no production caller
/// (S-06/S-06b). `CLAUDE.md` §3 rule 6 — this is stated here rather than
/// implied, so nobody reads the emit as evidence the store is checksumming
/// anything yet.
///
/// # Why `Error`, and why it is bounded by structure
///
/// One event per **block**, never per record: a block is 4,088 bytes, which is
/// 73 records at the current stride, so a wholly corrupt month of one-minute
/// bars is on the order of a hundred events rather than thousands. The bytes
/// themselves are never logged — only their length, both checksums, and which
/// block.
fn note_block_mismatch(header: &Header, block: u64, bytes: &[u8], stored: u32, computed: u32) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("store.block", "checksum mismatch")
            .with("block", telemetry::Value::Uint(block))
            .with("stored", telemetry::Value::Uint(u64::from(stored)))
            .with("computed", telemetry::Value::Uint(u64::from(computed)))
            .with("bytes", telemetry::Value::count(bytes.len()))
            .with("n_valid", telemetry::Value::Uint(header.n_valid))
            .with(
                "symbol_id",
                telemetry::Value::Uint(u64::from(header.symbol_id)),
            ),
    );
}

/// Verifies one block's committed bytes against its stored checksum.
///
/// # Errors
///
/// [`FormatError::ChecksumsAbsent`] when the header does not declare
/// checksums — "verified" and "there was nothing to verify against" are
/// different answers and this refuses to conflate them.
/// [`FormatError::BlockChecksum`] naming the block and both numbers when the
/// bytes are not the bytes that were sealed. Plus anything [`seal`] refuses.
///
/// # Examples
///
/// ```
/// # use store::{block, format::{FLAG_CHECKSUMS, FormatError}, header::Header, layout::Layout};
/// let v2 = Layout::V2;
/// let header = Header::genesis(1, 60, FLAG_CHECKSUMS).advance(1, 100, 100)?;
/// let record = [7u8; 56];
/// let sealed = block::seal(v2, header.n_valid, 0, &record)?;
/// assert_eq!(block::verify(&header, v2, 0, &record, sealed), Ok(()));
///
/// // The write was lost and the extent reads as zeros. The bar is "sane"
/// // and its open interest is a real zero; only the checksum knows.
/// assert!(block::verify(&header, v2, 0, &[0u8; 56], sealed).is_err());
/// # Ok::<(), FormatError>(())
/// ```
pub fn verify(
    header: &Header,
    layout: Layout,
    block: u64,
    bytes: &[u8],
    stored: u32,
) -> Result<(), FormatError> {
    verify_through(header, layout, block, bytes, &[], stored).map(|_| ())
}

/// A tail block whose stored checksum an interrupted append sealed past the
/// commit, on the rolling log.
///
/// # What happened, and why this is a `Warn`
///
/// `crate::file::BarFile::append` writes the records, seals the sidecar, and
/// only then writes the header slot that publishes them. A crash between the
/// last two leaves the tail block's entry computed over more records than the
/// header claims. Nothing is wrong with a committed byte: [`verify_through`]
/// has just shown that the stored checksum is the checksum of these committed
/// bytes followed by records the file really holds past the commit. The block
/// is served, so the line is the only trace of the interrupted append. It is
/// not an `Error`, because nothing is refused and nothing is lost.
///
/// One event per block verification that needed the proof, never per record.
fn note_interrupted_append(header: &Header, block: u64, sealed_through: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            "store.block",
            "tail block sealed past the commit by an interrupted append",
        )
        .with("block", telemetry::Value::Uint(block))
        .with("n_valid", telemetry::Value::Uint(header.n_valid))
        .with("sealed_through", telemetry::Value::Uint(sealed_through))
        .with(
            "symbol_id",
            telemetry::Value::Uint(u64::from(header.symbol_id)),
        ),
    );
}

/// Verifies one block, and admits a tail block whose checksum an interrupted
/// append sealed past the commit, **only on a positive proof**.
///
/// `bytes` is exactly what [`verify`] takes: the range
/// [`Layout::covered_byte_range`] names under `header.n_valid`. `past` is the
/// whole records the file holds straight after them, in file order. An empty
/// `past` makes this [`verify`], which is how [`verify`] is implemented.
///
/// Returns the record count the stored checksum covers: `header.n_valid`
/// when it matches the committed extent, and a larger count when it matches
/// one of the longer extents described below.
///
/// # Which longer extents count, and why a match is proof rather than hope
///
/// The candidates are the committed extent followed by one, two, … whole
/// records of `past`, and never past the block's nominal end. A block that is
/// already full has no room, so only a partial tail block can have one. A
/// partial trailing record is never a candidate, because no writer seals one.
///
/// A candidate matches only if the stored number is the CRC-32C of the
/// committed bytes **followed by** bytes the file holds. The committed bytes
/// are inside every candidate, so a flipped committed byte changes every
/// candidate's checksum. A CRC-32C detects every burst of up to 32 bits over
/// any length, so a flip confined to 32 bits cannot match the extent that was
/// really sealed. It can match another candidate only by a 32-bit collision.
/// A block with `k` whole records past its commit is compared `k + 1` times
/// instead of once, and `k` is at most 72 at the bar geometry. That is the
/// price of the proof, and `docs/06-limits.md` states it.
///
/// # Cost
///
/// One CRC-32C over `bytes`, then one extension per candidate record, so
/// every byte of the nominal block is read at most once. Bounded by
/// [`Layout::block_len`], not by the file.
///
/// # Errors
///
/// Everything [`verify`] refuses, unchanged: [`FormatError::ChecksumsAbsent`],
/// [`FormatError::BlockChecksum`] naming the checksum of the COMMITTED
/// extent, and anything [`seal`] refuses.
pub fn verify_through(
    header: &Header,
    layout: Layout,
    block: u64,
    bytes: &[u8],
    past: &[u8],
    stored: u32,
) -> Result<u64, FormatError> {
    if !header.checksums_present() {
        note_unverifiable(header, block);
        return Err(FormatError::ChecksumsAbsent);
    }
    let computed = seal(layout, header.n_valid, block, bytes)?;
    if computed == stored {
        return Ok(header.n_valid);
    }
    if let Some(through) =
        sealed_past_the_commit(layout, header.n_valid, bytes, past, computed, stored)
    {
        note_interrupted_append(header, block, through);
        return Ok(through);
    }
    note_block_mismatch(header, block, bytes, stored, computed);
    Err(FormatError::BlockChecksum {
        block,
        stored,
        computed,
    })
}

/// The longer extent `stored` was sealed over, when it was sealed over one.
///
/// `committed` is the checksum of `bytes`, which [`seal`] has already proved
/// is the block's covered range. The room left in the block is its nominal
/// length minus that range, so a full block has none and yields `None`
/// without reading `past` at all.
fn sealed_past_the_commit(
    layout: Layout,
    n_valid: u64,
    bytes: &[u8],
    past: &[u8],
    committed: u32,
    stored: u32,
) -> Option<u64> {
    // `unwrap_or(usize::MAX)` on both conversions is the same argument
    // `byte_count` makes: each can fail only where `usize` cannot hold one
    // block's length, and no target this workspace builds for is that narrow.
    // Where the stride saturated, `chunks_exact` would yield no record and the
    // answer would be `None`, the refusal.
    let stride = usize::try_from(layout.record_stride()).unwrap_or(usize::MAX);
    let room = usize::try_from(layout.block_len())
        .unwrap_or(usize::MAX)
        .saturating_sub(bytes.len());
    let within = past.get(..room).unwrap_or(past);
    let mut running = committed;
    for (record, through) in within.chunks_exact(stride).zip(n_valid.saturating_add(1)..) {
        running = crate::crc::crc32c_extend(running, record);
        if running == stored {
            return Some(through);
        }
    }
    None
}

/// How many bytes `block` covers under `n_valid`.
fn covered_len(layout: Layout, n_valid: u64, block: u64) -> Result<u64, FormatError> {
    let (start, end) = layout.covered_byte_range(block, n_valid)?;
    Ok(end - start)
}

/// The length of `bytes` as a `u64`, without a cast this workspace denies.
///
/// This **counted** the bytes one at a time until D-0032, to dodge a `usize`→
/// `u64` conversion whose failure arm no test could reach. That reasoning was
/// backwards: it bought a covered branch by walking the block a second time,
/// beside a checksum that walks it once. Measured before the change, on a full
/// 4,088-byte block, the fold cost 0.65 ns/byte — 2,625 ns, against 1,487 ns
/// for the whole checksum after D-0032. Retiring it is most of what makes
/// [`seal`] fast.
///
/// `unwrap_or(u64::MAX)` is not a fallback that hides anything: the conversion
/// can only fail on a target where `usize` is wider than 64 bits, and there the
/// answer `u64::MAX` is a length no block covers, so [`seal`] refuses with
/// [`FormatError::BlockLengthMismatch`] naming the real length. It is the same
/// loud refusal by a different route, never a silent pass.
fn byte_count(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}
