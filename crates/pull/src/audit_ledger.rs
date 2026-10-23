//! THE AUDIT LEDGER APPEND (IA-16; D-4888, D-4890, D-4906; RA-125, FM-127,
//! IA-GR-41): the one function every writer of `<store>/audit/<vendor>.<name>`
//! calls, so two programs appending one ledger at once never tear or join a
//! line and never wait for ever.
//!
//! The repair journal `audit/<vendor>.repairs` has three writers (the
//! back-fill door's `repairing` line, the repair's `repair` line and its
//! `refused` line, D-3350, WP-12d) and two kinds of process that write it at
//! once: the api's `/pull/repair` and `cli zerodha-repair` for Zerodha, and an
//! INDEX and a CASH repair-reason import for GDFL. Each append here:
//!
//! 1. refuses a ledger name that is not one lower-case word, and a store root
//!    that is not a directory (a writer never creates the root or an ancestor
//!    of it: an unplugged drive's mount point is not recreated, FM-125);
//! 2. takes `<store>/audit/<vendor>.<name>.lock` exclusive, waiting at most
//!    [`crate::gdfl_run::locks::LOCK_WAIT`] (D-4906), and past it refuses by
//!    name with the holder, never waiting for ever on a stalled drive;
//! 3. appends through the ledger's own torn-tail rule
//!    (`request_minutes::append_named`, R3-18): a last line with no line break
//!    is closed first, so a crash's fragment stays one malformed line every
//!    reader names and is never joined to the next; the line rules are
//!    checked; the write is synced.
//!
//! The gap ledger's writers (`audit/<vendor>.gaps`) move here with IA-10b,
//! which owns `request_minutes.rs`; until then they take the ledger file's own
//! `flock` (R3-18), which this append takes too, inside the lock above, so the
//! two never interleave a batch.
//!
//! # Cost
//!
//! O(1) per append in the ledger's size: one lock open and take, one 1-byte
//! read of the tail, at most one newline, one write and one `sync_data`
//! (bench C-GR-15, `appends_cost_the_same_at_any_ledger_length`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use brutex_core::vendor::Vendor;

use crate::gdfl_run::locks::{LOCK_WAIT, take_path_within};

/// The lock file of `<store>/audit/<vendor>.<name>`.
#[must_use]
pub fn lock_path(store_root: &Path, vendor: Vendor, name: &str) -> PathBuf {
    store_root
        .join("audit")
        .join(format!("{}.{name}.lock", vendor.as_str()))
}

/// Appends `lines` to `<store>/audit/<vendor>.<name>` under its lock (D-4888),
/// waiting at most [`LOCK_WAIT`] for it (D-4906).
///
/// # Errors
///
/// A name that is not one lower-case word; a store root that is not a
/// directory; the lock not taken within its wait (named with its holder) or
/// refused; the ledger's own refusal (a line breaking the line rules, the
/// host). Nothing is written on any of them.
pub fn append(
    store_root: &Path,
    vendor: Vendor,
    name: &str,
    lines: &[String],
) -> Result<(), String> {
    append_within(store_root, vendor, name, lines, LOCK_WAIT)
}

/// [`append`] with the lock's wait given: the test seam for a stalled holder.
pub(crate) fn append_within(
    store_root: &Path,
    vendor: Vendor,
    name: &str,
    lines: &[String],
    wait: Duration,
) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
        return Err(format!(
            "{name:?} is not an audit ledger name (one lower-case word); nothing was written"
        ));
    }
    // NEVER THE ROOT OR AN ANCESTOR OF IT (FM-125): a store root that is gone
    // (an unplugged drive) is named, and nothing is created in its place.
    if !store_root.is_dir() {
        return Err(format!(
            "{}: the store root is not a directory, so no audit ledger is written beneath it \
             (a writer never recreates a root that went away)",
            store_root.display()
        ));
    }
    let dir = store_root.join("audit");
    std::fs::create_dir_all(&dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let lock = lock_path(store_root, vendor, name);
    let _held = take_path_within(&lock, wait).map_err(|fault| {
        format!(
            "{}: {fault}; the ledger line was not written",
            lock.display()
        )
    })?;
    super::request_minutes::append_named(store_root, vendor, name, lines)
}

#[cfg(test)]
#[path = "audit_ledger_tests.rs"]
mod tests;
