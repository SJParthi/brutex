//! The advisory-lock guard: a lock this process holds is released by
//! `File::unlock`, never by closing a descriptor. D-0693.
//!
//! # The rule the standard library does not state
//!
//! `File::lock`, `lock_shared`, `try_lock` and `try_lock_shared` are `flock(2)`
//! locks, and a `flock` lock belongs to the **open file description**, not to
//! the descriptor that took it. Every descriptor that refers to that
//! description holds the same lock, and closing one of them releases nothing
//! while another is still open.
//!
//! On Linux `std::process::Command` duplicates the whole descriptor table into
//! the child, and a `CLOEXEC` descriptor is closed only when the child execs.
//! So a thread that spawns a child while another thread holds a lock leaves a
//! second reference to the locked description in that child until its exec.
//! A holder that releases only by dropping its `File` then leaves the lock
//! alive on the child's reference, and the next `try_lock` or
//! `try_lock_shared` of the same file refuses with `WouldBlock` — in this
//! process or in any other.
//!
//! That is not a hypothesis. CI hit it: the `cli` test
//! `checkpoint_counter_reseeding_and_changed_batch_boundaries_refuse` failed
//! with "checkpoint payload is busy or cannot be read" because the checkpoint
//! writer released its payload lock by drop and the reader's shared lock came
//! straight after. macOS applies `CLOEXEC` inside `posix_spawn` atomically, so
//! the race never reproduces there with a real spawn. `File::try_clone` makes
//! the same second reference deterministically, and every test that proves
//! this module uses it.
//!
//! An explicit `File::unlock` releases the lock on the description itself,
//! however many descriptors still refer to it. [`Flock`] is the one place that
//! call is made: on the success path by [`Flock::release`], which returns the
//! refusal to its caller, and on every other path by `Drop`, which cannot
//! return one and logs it instead.
//!
//! # Why this lives in `store`
//!
//! Every crate that takes a production file lock — `store`, `pull`, `api` and
//! `cli` — already depends on this one, so the guard adds no arrow to the crate
//! graph. `store` also owns the busiest lock user, [`crate::file::BarFile`]'s
//! month lock, and already emits through `telemetry`, which the drop note
//! needs.
//!
//! # What it deliberately does not do
//!
//! It implements no `io::Read` and no `io::Write`. `File::flush` is a no-op, so
//! a delegating `flush` would carry a mutant no test can kill. A caller reads
//! and writes through `Deref`/`DerefMut`, or through a duplicate descriptor
//! taken before the lock (`Flock::lock(file.try_clone()?, path)`) when it must
//! also move or wrap the `File` while the lock is held. That duplicate names the
//! same open file description, so its guard takes and releases the same lock
//! the original handle sees.

use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::fs::{File, TryLockError};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

/// A `flock(2)` lock this process holds on one open file description.
///
/// It is released by `File::unlock`, never by close, so a duplicate descriptor
/// left in a child — or made by `try_clone` — cannot keep it alive.
///
/// `F` is the file, owned (`File`) or borrowed (`&File`, `&mut File`). `P` is
/// the path the lock is named by in a refusal: owned by default, borrowed
/// (`&Path`) where a per-call guard should allocate nothing.
#[derive(Debug)]
#[must_use = "dropping a Flock unlocks immediately"]
pub struct Flock<F: Borrow<File>, P: AsRef<Path> = PathBuf> {
    /// The file whose open file description holds the lock.
    file: F,
    /// The path a refusal to release names.
    path: P,
    /// Whether `Drop` still owes the unlock. Cleared by [`Flock::release`]
    /// before it unlocks, so a refused release is reported once, by the
    /// release, and never again by `Drop`.
    held: bool,
}

impl<F: Borrow<File>, P: AsRef<Path>> Flock<F, P> {
    /// Blocks until the exclusive lock is taken.
    ///
    /// # Errors
    ///
    /// The host's refusal, exactly as `File::lock` returns it.
    pub fn lock(file: F, path: P) -> io::Result<Self> {
        file.borrow().lock()?;
        Ok(Self::taken(file, path))
    }

    /// Blocks until a shared lock is taken.
    ///
    /// # Errors
    ///
    /// The host's refusal, exactly as `File::lock_shared` returns it.
    pub fn lock_shared(file: F, path: P) -> io::Result<Self> {
        file.borrow().lock_shared()?;
        Ok(Self::taken(file, path))
    }

    /// Takes the exclusive lock, or refuses without waiting.
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` while another description holds the lock, or
    /// the host's refusal, exactly as `File::try_lock` returns them.
    pub fn try_lock(file: F, path: P) -> Result<Self, TryLockError> {
        file.borrow().try_lock()?;
        Ok(Self::taken(file, path))
    }

    /// Takes a shared lock, or refuses without waiting.
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` while another description holds the lock
    /// exclusively, or the host's refusal, exactly as `File::try_lock_shared`
    /// returns them.
    pub fn try_lock_shared(file: F, path: P) -> Result<Self, TryLockError> {
        file.borrow().try_lock_shared()?;
        Ok(Self::taken(file, path))
    }

    /// Releases the lock on the success path, and says so if it cannot.
    ///
    /// `Drop` performs the same unlock on every other path, but it has nowhere
    /// to send a refusal except the log. A caller that is about to report
    /// success calls this instead, so a lock that could not be released turns
    /// the success into a named failure rather than into a line nobody reads.
    ///
    /// # Errors
    ///
    /// [`Unreleased`], naming the lock's path and the host's refusal. The guard
    /// is consumed either way and `Drop` does not try again.
    pub fn release(mut self) -> Result<(), Unreleased> {
        self.held = false;
        unlock(self.file.borrow()).map_err(|why| Unreleased {
            path: self.path.as_ref().to_path_buf(),
            why,
        })
    }

    /// The guard for a lock the caller has just taken.
    const fn taken(file: F, path: P) -> Self {
        Self {
            file,
            path,
            held: true,
        }
    }
}

impl<F: Borrow<File>, P: AsRef<Path>> Deref for Flock<F, P> {
    type Target = File;

    fn deref(&self) -> &File {
        self.file.borrow()
    }
}

impl<F: BorrowMut<File>, P: AsRef<Path>> DerefMut for Flock<F, P> {
    fn deref_mut(&mut self) -> &mut File {
        self.file.borrow_mut()
    }
}

impl<F: Borrow<File>, P: AsRef<Path>> Drop for Flock<F, P> {
    /// Every path that did not call [`Flock::release`]: an early return, a
    /// refusal, an owner that dropped. The unlock is explicit here for the
    /// reason the module header gives, and a refusal is logged because a drop
    /// cannot return one.
    fn drop(&mut self) {
        if self.held
            && let Err(why) = unlock(self.file.borrow())
        {
            note_unreleased(self.path.as_ref(), &why);
        }
    }
}

/// A lock whose unlock the host refused.
///
/// Returned by [`Flock::release`]. The lock may still be held by the open file
/// description, so a later attempt to take it can refuse until every descriptor
/// referring to that description has closed. For a guard that owned its
/// `File`, that is the guard's own descriptor, which closes as the guard is
/// consumed, and any duplicate of it. For a guard over a borrowed file, the
/// guard closes nothing: the owner's descriptor is one of those, and the lock
/// lasts at least as long as the owner keeps it open.
#[derive(Debug)]
pub struct Unreleased {
    /// The file the lock was taken on.
    pub path: PathBuf,
    /// The host's refusal to unlock it.
    pub why: io::Error,
}

impl fmt::Display for Unreleased {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the advisory lock on {} could not be released, and it stays held until every \
             descriptor of its open file description has closed: {}",
            self.path.display(),
            self.why
        )
    }
}

impl std::error::Error for Unreleased {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.why)
    }
}

impl From<Unreleased> for io::Error {
    /// Keeps the host's kind, and carries the whole sentence so the path
    /// survives into a caller that reports only an `io::Error`.
    fn from(unreleased: Unreleased) -> Self {
        Self::new(unreleased.why.kind(), unreleased.to_string())
    }
}

/// The one `File::unlock` this crate makes on a guard's behalf.
///
/// Under `cfg(test)` a one-shot, per-thread refusal of a chosen kind can be
/// armed first, because no real file refuses an unlock on request and the
/// refusal arms of [`Flock::release`] and `Drop`, and of the callers in this
/// crate that turn a refused release into their own error, must still be
/// driven.
fn unlock(file: &File) -> io::Result<()> {
    #[cfg(test)]
    if let Some(kind) = tests::take_unlock_refusal() {
        return Err(io::Error::new(
            kind,
            "unlock refused by the flock test seam",
        ));
    }
    file.unlock()
}

/// A guard dropped without a release, whose unlock the host refused.
///
/// `Warn`, not `Error`: the kernel still frees the lock once every descriptor
/// of its open file description has closed. For a guard that owned its `File`
/// that is its own descriptor, which closes as the guard drops, and any
/// duplicate of it. For a guard over a borrowed file the drop closes nothing,
/// and the owner's descriptor holds the lock for as long as the owner keeps
/// it open. What the line records is that the lock outlived its guard, which
/// is what turns a later acquisition into a `WouldBlock` nobody can otherwise
/// explain. One event per refused unlock, never per acquisition, so the normal
/// path costs nothing beyond the `flock` call itself.
fn note_unreleased(path: &Path, why: &io::Error) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.flock", "advisory lock not released by its guard")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

#[cfg(test)]
#[path = "flock_tests.rs"]
pub(crate) mod tests;
