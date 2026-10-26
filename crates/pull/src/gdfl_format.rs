//! A vendor format change in either GDFL reader, said ONCE PER SHAPE
//! (ERR-11, R4-22, D-7312).
//!
//! Both readers refuse a first line they do not know by its shape
//! ([`crate::gdfl_cm::HeaderShape`]: the BLAKE3 of its column names and their
//! count) under the open cause `header_unknown`, one `refused` line per file.
//! This module is the one line a reader is told once per shape, naming the
//! first member that showed it, and the one memory that keeps it to once:
//! a process-wide set of at most [`MAX_SHAPES_REMEMBERED`] digests behind one
//! lock. It is its own module because the capital-market reader's day-folder
//! lookup holds no hash table but the scope's (CM-13,
//! `gdfl_cm::tests::day_folder_lookup_is_one_index_into_a_ticker_slot_array`),
//! and this memory is no part of that lookup.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::gdfl_cm::HeaderShape;
use crate::session::Day;

/// The target of a vendor format change's own line (ERR-11, D-7312).
pub const FORMAT_TARGET: &str = "pull.gdfl.format";

/// The most distinct header shapes [`note_format_changed`] remembers in one
/// process. Past it a new shape is still said, every time: the memory is
/// bounded and the line is never dropped, so an archive of a thousand
/// garbage headers costs a thousand lines, never a growing set.
pub const MAX_SHAPES_REMEMBERED: usize = 1_024;

/// The shapes already said in this process.
static SHAPES_SAID: OnceLock<Mutex<HashSet<[u8; 32]>>> = OnceLock::new();

/// Whether `digest` is said now: the first time `seen` meets it, or every
/// time once `seen` holds [`MAX_SHAPES_REMEMBERED`] shapes (bounded memory,
/// never a dropped line). One probe and at most one insert.
pub(crate) fn first_sight(seen: &mut HashSet<[u8; 32]>, digest: [u8; 32]) -> bool {
    if seen.len() < MAX_SHAPES_REMEMBERED {
        seen.insert(digest)
    } else {
        !seen.contains(&digest)
    }
}

/// Says a vendor format change ONCE PER SHAPE (ERR-11, R4-22, D-7312): for
/// the `shape` of an unknown header ([`CmRefusal::HeaderUnknown`], or the
/// options reader's own), one `error` line at [`FORMAT_TARGET`] naming the
/// tree, the day, the FIRST member that showed the shape, its column count
/// and its digest. Every later file of the
/// same shape is still refused by name, by the import's own `refused` line,
/// whose words carry the same digest; this line is the one a reader is told
/// once. `error`, so it is in the durable audit stream and searchable on
/// `/logs` by the digest. Returns whether the line was written (the first
/// time this process saw the shape, or past [`MAX_SHAPES_REMEMBERED`]).
///
/// The durable ALERT keyed by the digest is R4-05's
/// (`vendor_format_changed`, `pull::alerts::KINDS`): this build holds no such
/// kind, so no queue record is written here, and the cut link of 18 Oct
/// 2026 leaves the kind to that package. O(1): one lock, one probe.
#[must_use]
pub fn note_format_changed(tree: &str, day: Day, member: &str, shape: &HeaderShape) -> bool {
    note_shape(tree, day, member, shape, "header_unknown")
}

/// [`note_format_changed`] for a column-set change under a known header
/// (ERR-11, D-7312 item 5: [`crate::gdfl_cm::CmRefusal::ColumnsChanged`], or
/// the options reader's own): the same once-per-shape line and memory, its
/// code `columns_changed` and its shape the rows' ([`HeaderShape::of_rows`],
/// never equal to a header's own shape).
#[must_use]
pub fn note_columns_changed(tree: &str, day: Day, member: &str, shape: &HeaderShape) -> bool {
    note_shape(tree, day, member, shape, "columns_changed")
}

/// The one line of a vendor format change, `code` its refusal's code.
fn note_shape(tree: &str, day: Day, member: &str, shape: &HeaderShape, code: &str) -> bool {
    let said = SHAPES_SAID.get_or_init(Default::default);
    let first = match said.lock() {
        Ok(mut seen) => first_sight(&mut seen, shape.digest),
        // A poisoned set is a panic elsewhere; the line is said rather than
        // lost (never silent).
        Err(_) => true,
    };
    if !first {
        return false;
    }
    let day_text = day.to_string();
    let hex = shape.hex();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(FORMAT_TARGET, "vendor format changed")
            .with("feed", "gdfl")
            .with("tree", tree)
            .with("day", day_text.as_str())
            .with("member", member)
            .with("columns", shape.columns)
            .with("shape", hex.as_str())
            .with("code", code),
    );
    true
}
