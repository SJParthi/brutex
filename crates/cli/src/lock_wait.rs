//! A bounded wait for a non-blocking lock that a short-lived holder refuses
//! (cli1-2, cli1-3, expr-1, expr-2, indexstop-2; D-2620).
//!
//! # Why a writer now asks again
//!
//! Four writers took their lock with one `try_lock` and refused at the first
//! `WouldBlock`: the invocation index (`operation_audit::begin`), the store's
//! execution lease (`execution_lease::Lease::acquire`), a search checkpoint's
//! owner (`search_checkpoint::Journal::open`) and a Boolean publication's
//! owner (`boolean_candidate_persistence::prepare_in_namespace`). Each of
//! those locks is also taken, for microseconds, by a reader or a probe: the
//! browser's audited GET routes, `/backtest/run.json`'s lease probe, a
//! checkpoint snapshot's shared probe and a dashboard's read lease. A real
//! sweep therefore failed, or a resumed rung burned its checkpoints, because a
//! page polled at the wrong instant.
//!
//! # The bound
//!
//! [`patiently`] asks again only on `WouldBlock`, every [`WAIT`] for at most
//! [`WAITS`] times: one second in all, a constant no input raises. A real
//! owner holds each of these locks for its whole run (minutes), so it is
//! still refused, one second later than before; a host refusal
//! (`TryLockError::Error`) is never retried. A reader that outlasts the bound
//! is still refused by name: the bound narrows the window, it does not remove
//! it, and `docs/06-limits.md` says so.

use std::fs::TryLockError;
use std::time::Duration;

/// How many times a `WouldBlock` is asked again before it is the answer.
pub(crate) const WAITS: u32 = 50;

/// The pause before each repeat; `WAITS × WAIT` is one second.
pub(crate) const WAIT: Duration = Duration::from_millis(20);

/// Runs `attempt` until it takes the lock, refuses with anything other than
/// `WouldBlock`, or has been refused `WouldBlock` [`WAITS`] + 1 times.
///
/// `attempt` must hand back an owned handle each call (open the file, or
/// `try_clone` an open one): a refused `try_lock` leaves nothing held, and the
/// handle it consumed is closed with the refusal.
///
/// # Errors
///
/// The last attempt's refusal: `WouldBlock` once the bound is spent, or the
/// first host error at once.
pub(crate) fn patiently<T>(
    attempt: impl FnMut() -> Result<T, TryLockError>,
) -> Result<T, TryLockError> {
    within(WAITS, WAIT, attempt)
}

/// [`patiently`] with an explicit bound, for tests that must not sleep a
/// second to prove the bound is finite.
pub(crate) fn within<T>(
    waits: u32,
    wait: Duration,
    mut attempt: impl FnMut() -> Result<T, TryLockError>,
) -> Result<T, TryLockError> {
    let mut waited = 0_u32;
    loop {
        match attempt() {
            Err(TryLockError::WouldBlock) if waited < waits => {
                waited += 1;
                std::thread::sleep(wait);
            }
            other => return other,
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "scratch lock fixtures must fail loudly"
)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs::{File, OpenOptions};
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "brutex-lock-wait-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        dir.join("owner.lock")
    }

    fn open(path: &std::path::Path) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .expect("open scratch lock")
    }

    /// Every count of `WouldBlock` from 0 to the bound + 1, against bounds 0,
    /// 1 and 3: an attempt is made exactly `min(blocked, waits) + 1` times,
    /// the lock is taken iff `blocked <= waits`, and the refusal after the
    /// bound is `WouldBlock` itself.
    #[test]
    fn would_block_is_asked_again_exactly_up_to_the_bound() {
        for waits in [0_u32, 1, 3] {
            for blocked in 0..=waits + 2 {
                let calls = Cell::new(0_u32);
                let answer = within(waits, Duration::ZERO, || {
                    calls.set(calls.get() + 1);
                    if calls.get() <= blocked {
                        Err(TryLockError::WouldBlock)
                    } else {
                        Ok(calls.get())
                    }
                });
                assert_eq!(calls.get(), blocked.min(waits) + 1, "{waits}/{blocked}");
                if blocked <= waits {
                    assert_eq!(answer.ok(), Some(blocked + 1), "{waits}/{blocked}");
                } else {
                    assert!(
                        matches!(answer, Err(TryLockError::WouldBlock)),
                        "{waits}/{blocked}"
                    );
                }
            }
        }
    }

    /// A host refusal is the answer at once, before or after a `WouldBlock`:
    /// it is never mistaken for contention and never waited on.
    #[test]
    fn a_host_error_is_returned_at_once_and_never_retried() {
        for after in 0_u32..3 {
            let calls = Cell::new(0_u32);
            let answer: Result<(), _> = within(10, Duration::ZERO, || {
                calls.set(calls.get() + 1);
                if calls.get() <= after {
                    Err(TryLockError::WouldBlock)
                } else {
                    Err(TryLockError::Error(std::io::Error::other("injected EIO")))
                }
            });
            assert_eq!(calls.get(), after + 1);
            assert!(
                matches!(&answer, Err(TryLockError::Error(why)) if why.to_string().contains("injected EIO")),
                "a host error must be returned as itself: {answer:?}"
            );
        }
    }

    /// The real lock: a holder that lets go inside the bound is waited for,
    /// and one that does not is still refused `WouldBlock`.
    #[test]
    fn a_holder_released_inside_the_bound_is_waited_for_and_one_outlasting_it_is_refused() {
        let path = scratch("release");
        let holder = open(&path);
        holder.lock_shared().expect("a reader's shared lock");
        let released = std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(60));
                holder.unlock().expect("the reader lets go");
            });
            let taken = patiently(|| {
                let file = open(&path);
                file.try_lock().map(|()| file)
            });
            worker.join().expect("holder thread");
            taken
        });
        let taken = released.expect("the writer waits out a short reader");
        taken.unlock().expect("release");
        drop(taken);
        let stuck = open(&path);
        stuck.lock().expect("an owner's exclusive lock");
        let refused = within(2, Duration::from_millis(1), || {
            let file = open(&path);
            file.try_lock().map(|()| file)
        });
        assert!(matches!(refused, Err(TryLockError::WouldBlock)));
        stuck.unlock().expect("release");
    }

    #[test]
    fn the_bound_is_one_second() {
        assert_eq!(WAIT * WAITS, Duration::from_secs(1));
    }
}
