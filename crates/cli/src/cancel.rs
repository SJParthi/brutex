//! A process-wide, cooperative stop for engine work (audit-20261003
//! hunt-api-2, D-1551).
//!
//! # Why it exists
//!
//! `api` runs a sweep, a descent or a command on a blocking thread. Until this
//! module a running sweep had no point at which it could be asked to stop, so a
//! shutdown either waited out the whole sweep (hours) or abandoned it mid-run
//! (D-1582). `api` now calls [`request`] when it stops; the work below checks
//! it at its STRUCTURAL boundaries and ends there with [`CANCELLED`] in its
//! answer.
//!
//! # Where it is checked, and where it is not
//!
//! At every stored instrument-month the loaders open (`stored`'s classified
//! loader and `fold_audit::read_month`), per candidate of the exit-grid screen,
//! once more before a range table is rendered, and at every single-stop
//! timeframe boundary. NEVER inside the per-bar folds of `vocab`, `engine`,
//! `indicators` or `runner`: gate 17's rule is that their innermost loops call
//! nothing at all, and this crate cannot reach into them anyway. The cost of a
//! check is one `Acquire` load of an atomic flag.
//!
//! # Why process-wide, and why it never resets
//!
//! The only caller is a process that is ending. A stop asked for once applies
//! to every engine task still running in it, and nothing new is started after
//! it. A reset would let a later run complete over months an earlier stop had
//! already refused, so there is none.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Set once, by [`request`]; never cleared.
static STOP: AtomicBool = AtomicBool::new(false);

/// How many boundary checks found the stop and refused to continue.
static OBSERVED: AtomicU64 = AtomicU64::new(0);

/// The sentence every cancelled engine answer begins with.
///
/// Loud on purpose: a cancelled run's partial output is never a result, and
/// this is what keeps it from being read as one.
pub const CANCELLED: &str = "CANCELLED: the process was asked to stop while this engine work \
     ran, so it stopped at its next structural boundary. Its result is NOT complete and is \
     not recorded as one. Work may have reached the append-only result files before the \
     stop; inspect them before retrying, then run it again.";

/// Asks every engine task in this process to stop at its next boundary.
///
/// Idempotent. There is deliberately no way to withdraw it (see the module
/// documentation).
pub fn request() {
    STOP.store(true, Ordering::Release);
}

/// Whether [`request`] has been called in this process.
#[must_use]
pub fn requested() -> bool {
    STOP.load(Ordering::Acquire)
}

/// How many boundary checks have refused because of a stop.
///
/// A test reads it to prove a run stopped AT a boundary rather than running to
/// the end; an operator never needs it.
#[must_use]
pub fn observed() -> u64 {
    OBSERVED.load(Ordering::Acquire)
}

/// The boundary check: `Ok` while no stop was asked for.
///
/// `at` names the boundary and is only built when the check refuses.
///
/// # Errors
///
/// [`CANCELLED`] followed by where the work stopped, once [`request`] has run.
pub fn check(at: impl FnOnce() -> String) -> Result<(), String> {
    if requested() {
        OBSERVED.fetch_add(1, Ordering::AcqRel);
        return Err(format!("{CANCELLED} Stopped at: {}.", at()));
    }
    Ok(())
}
