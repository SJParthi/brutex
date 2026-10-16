//! The fixed-stride bar file: bytes on disk, addressed by arithmetic.
//!
//! `docs/02-store-format.md` is the authority for every byte here.
//!
//! # What is here
//!
//! | Module | Owns |
//! |---|---|
//! | [`crc`] | CRC-32C, the one checksum this store computes |
//! | [`mod@format`] | the record, the shared constants, and every error |
//! | [`layout`] | one version's geometry, and the dispatch that selects it |
//! | [`header`] | the two-slot header and the single-write commit |
//! | [`block`] | the per-block checksum, over the committed prefix |
//! | [`path`] | the only way a store path is built |
//! | [`catalog`] | what the store HOLDS, walked back off the tree |
//! | [`mod@file`] | the bytes actually reaching the disk, and every refusal |
//! | [`flock`] | the advisory-lock guard, released by unlock and never by close |
//! | [`provenance`] | where a month's bars came from: the `.prv` sidecar (D-3334) |
//! | [`sort_index`] | a month's rows ordered by each sortable column: the `.srt` sidecar (D-3349) |
//! | [`verdict`] | what verification found: day images and year indexes (D-4808) |
//! | [`staging`] | a writer's temporary, born without opening anything already there (D-4922) |
//!
//! # Seven properties the types enforce rather than document
//!
//! 1. **A record never straddles a checksum block.** The block length is a
//!    whole multiple of the record stride, so verifying a record reads exactly
//!    one block's checksum — never one of the two blocks its bytes fell into.
//!    See [`layout`].
//! 2. **The stride is never a global constant on the read path.** It comes
//!    from [`layout::Layout`], which comes from the file's own
//!    `format_version`. An unrecognised version is refused by number, and a
//!    retired one by name; nothing falls back to the current stride. See
//!    [`layout::Layout::for_version`].
//! 3. **A geometry is never mutated in place.** The header region, the field
//!    offsets, the checksum and the block length all changed, so this is
//!    format **version 2** with magic `BRUTEXB2`. Version 1's number is
//!    retired, never reused — `CLAUDE.md` §3 rule 8. See
//!    [`format::RETIRED_VERSIONS`]. Version 3 (`BRUTEXB3`, D-1571) keeps
//!    version 2's geometry byte for byte and makes block checksums
//!    mandatory; it is the version written now, and a version-2 file is
//!    still read and never rewritten.
//! 4. **A header update is one write of one self-checked unit.** There is no
//!    API that can update the counter, the timestamp and the checksum
//!    separately, because [`header::Commit`] carries one offset and one
//!    buffer. See [`header`].
//! 5. **Redundant header slots sit in different failure units.** Storage does
//!    not fail at byte granularity, so the two slots are a whole page apart
//!    rather than adjacent. See [`format::SLOT_STRIDE`].
//! 6. **A vendor cannot write outside its own prefix, lexically.** The first
//!    path segment is a `Vendor`, not a string, and every other segment is
//!    checked and case-canonical before a path exists. What that does *not*
//!    cover — a symlink — is stated in [`path::StorePath::to_path_buf`]
//!    rather than implied away.
//! 7. **A writer cannot lose a byte quietly.** Every write is an ordinary
//!    positional syscall, never a mapping — a mapping raises `SIGBUS` on a full
//!    disk and a signal cannot be caught — and every host refusal becomes a
//!    named [`file::StoreError`] carrying the path and the operation. A write
//!    that accepts zero bytes is refused rather than retried forever. See
//!    [`mod@file`].

#![forbid(unsafe_code)]

pub mod block;
pub mod catalog;
pub mod checksum_audit;
pub mod crc;
pub mod file;
/// The advisory-lock guard: every lock with an early return or an owner is
/// released by `File::unlock`, never by close (D-0693).
pub mod flock;
pub mod format;
/// Which generation of a bar month every reader opens: the original, or a
/// repair revision promoted by an append-only pointer (D-3350, WP-13c).
pub mod generation;
pub mod header;
pub mod layout;
/// A month's summary — its minute and second index, extremes, bars per day
/// and the input faults a gap audit refuses on — kept in a `.sum` sibling
/// while the month is written and in the `.sbar` summary section once sealed,
/// so a gap audit reads one summary a month and no bar (R3-06, D-4980).
pub mod month_summary;
/// Per-architecture `O_NOFOLLOW` / `O_NONBLOCK`, written once (D-0980).
pub mod open_flags;
pub mod path;
/// Where a month's bars came from: one append-only `.prv` record per landing,
/// written after the bars it names are committed and never touching them
/// (D-3334; DB-078, DB-G56).
pub mod provenance;
pub mod repair;
/// One way to open a file for reading (no link followed, never a wait,
/// regular files only) and the writer's refusal of a store whose `bars` is a
/// link (R3-16; RA-317, RA-298).
pub mod safe_open;
/// The sealed form of a closed month, `.sbar` version 1: a 512-byte head, a
/// keyed block directory, a time table, the packed `.prv` image and codec 1
/// frames, built once and read by position (D-4965, SC-2).
pub mod sealed;
/// Codec 1 of a sealed month: one frame of at most 256 bars as seven
/// bit-packed column streams, bit for bit for any seven `i64` (D-4939, SC-1).
pub mod sealed_codec;
pub mod sort_index;
/// A writer's temporary file, created `O_EXCL` so a name that already exists
/// (a hard link planted there, a dead writer's leftover) is never opened,
/// truncated or written through (RA-290, RA-287; D-4922).
pub mod staging;
pub mod time_index;
pub mod verdict;

/// Proof that this crate's nine `telemetry::emit` sites — `store.flock`'s
/// unreleased-lock note among them (D-0693) — reach a file, driven
/// through the production calls that own them. Compiled only under `cfg(test)`
/// — it installs the process-wide sink, which is a property of the test binary
/// and not of the store.
#[cfg(test)]
mod emits;

/// Test-only: re-runs a test where the permission bits bind, even as root. The
/// same file serves this crate's integration tests. D-0995, D-1485.
#[cfg(all(test, unix))]
#[path = "../tests/support/mod.rs"]
mod support;
