#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-PULL-PS1 (D-7335): the two refusals of the cash-session cache's
//! day lock no test reached at tip 2e7a0b6d3b56 (Route B), driven by direct
//! calls: a lock path with nothing at it once it was opened, and a lock
//! descriptor the host will not `flock` (Linux `O_PATH`). Through
//! `open_lock` the first is reached only when another process removes the
//! file between the open and the check, and `hold` is only ever handed the
//! descriptor `open_lock` opened itself. And a publication the host will not
//! write or sync, handed to `write_new_with`. No vendor byte is used.

use super::*;
use crate::gdfl_fixtures::scratch;

/// A lock path with nothing at it once it was opened is refused as gone,
/// naming it, never taken for a fresh lock; a regular file there is the
/// lock. The check `open_lock` makes after its open, driven directly.
#[test]
fn a_lock_path_with_nothing_at_it_once_opened_is_refused_as_vanished() {
    let root = scratch("ps1-cash-lock-vanished");
    let lock = root.join("2026-08-03.lock");
    assert_eq!(
        still_regular(&lock),
        Err(format!(
            "UNVERIFIED cache lock {} vanished while it was opened",
            lock.display()
        ))
    );
    fs::write(&lock, b"").unwrap();
    assert_eq!(still_regular(&lock), Ok(()));
    fs::remove_dir_all(root).unwrap();
}

/// A descriptor no lock call accepts: Linux `O_PATH`
/// (`include/uapi/asm-generic/fcntl.h`: `O_PATH 010000000`, which neither
/// x86-64 nor arm64 overrides), on which `flock` answers EBADF, as root too.
#[cfg(target_os = "linux")]
fn unlockable(at: &Path) -> File {
    use std::os::unix::fs::OpenOptionsExt as _;
    const O_PATH: i32 = 0o10_000_000;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_PATH)
        .open(at)
        .unwrap()
}

/// A day's lock descriptor the host will not `flock` is refused at once, on
/// either side, as UNVERIFIED and in the host's own words, naming the lock:
/// never waited on as a lock another holder keeps, never read as absent.
/// Linux only (`O_PATH`).
#[cfg(target_os = "linux")]
#[test]
fn a_lock_descriptor_the_host_will_not_flock_is_refused_in_its_words() {
    let root = scratch("ps1-cash-lock-unflockable");
    let lock = root.join("2026-08-03.lock");
    fs::write(&lock, b"").unwrap();
    // The premise, in the host's own words for each side.
    let Err(TryLockError::Error(shared)) = unlockable(&lock).try_lock_shared() else {
        panic!("the host took a shared flock on an O_PATH descriptor");
    };
    let Err(TryLockError::Error(exclusive)) = unlockable(&lock).try_lock() else {
        panic!("the host took an exclusive flock on an O_PATH descriptor");
    };
    for (kind, host) in [(Hold::Shared, shared), (Hold::Exclusive, exclusive)] {
        assert_eq!(
            hold(&unlockable(&lock), &lock, kind, LOCK_WAIT).err(),
            Some(format!(
                "UNVERIFIED cash-session cache lock {} refused by the host: {host}",
                lock.display()
            )),
            "{kind:?}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

/// A publication the host will not write or sync is refused as incomplete,
/// naming its path, and the file it began is retained, never removed and
/// never overwritten: `write_new` refuses the name the next time. The write
/// and sync `write_new` hands `write_new_with`, refused by a test.
#[test]
fn a_publication_the_host_will_not_write_is_retained_and_named() {
    let root = scratch("ps1-cash-write-refused");
    let path = root.join("2026-08-03.csv.gz");
    let refused = write_new_with(&path, b"ZQ invented", &|_, _| {
        Err(std::io::Error::other("ZQ invented: no room"))
    });
    assert_eq!(
        refused,
        Err(format!(
            "incomplete publication at {} retained: ZQ invented: no room",
            path.display()
        ))
    );
    assert!(path.is_file(), "the begun file is retained");
    let again = write_new(&path, b"other").unwrap_err();
    assert!(again.starts_with("cannot create "), "{again}");
    assert_eq!(fs::read(&path).unwrap(), b"");
    fs::remove_dir_all(root).unwrap();
}
