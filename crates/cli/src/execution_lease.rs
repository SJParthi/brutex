//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! One nonblocking execution lease per canonical store for cooperating CLI and
//! HTTP sweep entry points. This is admission authority, not a result receipt.
//!
//! A probe opens one fixed path and checks one OS lock. It reads no history and
//! allocates no storage proportional to bars, candidates or previous commands.
//! OS/file latency is not constant-time. Older binaries and callers that bypass
//! the entry points do not participate; their telemetry must remain visible.
//! The empty lock file is never removed or replaced by this protocol.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

const NAME: &str = ".sweep-execution-v1.lock";

/// A current inability to claim the store, distinct from a command outcome.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A cooperating writer owns the store's execution slot.
    Busy,
    /// The fixed lock path or its store could not be safely inspected.
    Unavailable(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => out.write_str(
                "another sweep owns this store's execution lease; no new work was queued",
            ),
            Self::Unavailable(why) => {
                write!(out, "the store execution lease is unavailable: {why}")
            }
        }
    }
}

fn unavailable(why: impl std::fmt::Display) -> Refusal {
    Refusal::Unavailable(why.to_string())
}

fn path(root: &Path) -> Result<PathBuf, Refusal> {
    let root = fs::canonicalize(root).map_err(unavailable)?;
    if !root.is_dir() {
        return Err(unavailable(
            "the configured store is not an existing directory",
        ));
    }
    Ok(root.join(NAME))
}

fn options() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = OpenOptions::new();
    // Same no-follow/nonblocking flags as operation_audit's native file door.
    #[cfg(target_os = "macos")]
    options.custom_flags(0x100 | 0x4);
    #[cfg(target_os = "linux")]
    options.custom_flags(0x20000 | 0x800);
    options
}

fn lock(file: &File) -> Result<(), Refusal> {
    file.try_lock().map_err(|why| match why {
        fs::TryLockError::WouldBlock => Refusal::Busy,
        fs::TryLockError::Error(why) => unavailable(why),
    })
}

fn verify(file: &File, path: &Path) -> Result<(), Refusal> {
    use std::os::unix::fs::MetadataExt as _;
    let held = file.metadata().map_err(unavailable)?;
    let named = fs::symlink_metadata(path).map_err(unavailable)?;
    if !held.is_file()
        || held.len() != 0
        || held.nlink() != 1
        || !named.is_file()
        || (held.dev(), held.ino()) != (named.dev(), named.ino())
    {
        return Err(unavailable(
            "the execution lock is not the same empty regular file; nothing was repaired",
        ));
    }
    Ok(())
}

/// Owns the exclusive OS lease until normal completion, unwind or process exit.
/// Dropping releases the descriptor; it never deletes the persistent lock path.
#[derive(Debug)]
#[must_use = "retain the lease throughout computation and terminal audit"]
pub struct Lease {
    _file: File,
}

impl Lease {
    /// Atomically claims the one store slot. It never waits for another writer.
    ///
    /// # Errors
    /// Refuses a held lease, an unavailable store, or an unsafe/nonempty lock
    /// path. No command is queued and no existing file is repaired.
    pub fn acquire(root: &Path) -> Result<Self, Refusal> {
        let path = path(root)?;
        let file = options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(unavailable)?;
        lock(&file)?;
        verify(&file, &path)?;
        Ok(Self { _file: file })
    }
}

/// Checks current admission without creating a file or changing a result.
/// Availability can change immediately; every POST must acquire its own lease.
///
/// # Errors
/// Refuses a held lease, an unavailable store, or an unsafe/nonempty lock
/// path. A missing lock file in an existing store is an available snapshot.
pub fn probe(root: &Path) -> Result<(), Refusal> {
    let path = path(root)?;
    let file = match options().read(true).open(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(why) => return Err(unavailable(why)),
    };
    lock(&file)?;
    verify(&file, &path)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "private concurrency fixtures must fail loudly"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "brutex-execution-lease-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("private lease fixture");
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_probe_is_read_only_and_a_lease_excludes_until_drop() {
        let root = Scratch::new();
        assert_eq!(probe(&root.0), Ok(()));
        assert!(!root.0.join(NAME).exists());
        let lease = Lease::acquire(&root.0).expect("first owner");
        assert!(matches!(Lease::acquire(&root.0), Err(Refusal::Busy)));
        assert_eq!(probe(&root.0), Err(Refusal::Busy));
        drop(lease);
        assert_eq!(probe(&root.0), Ok(()));
        assert!(root.0.join(NAME).exists());
        assert!(Lease::acquire(&root.0).is_ok());
    }

    #[test]
    fn parallel_contenders_cannot_share_the_same_slot() {
        let root = Scratch::new();
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let contenders: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        let result = Lease::acquire(&root.0);
                        barrier.wait();
                        result.is_ok()
                    })
                })
                .collect();
            assert_eq!(
                contenders
                    .into_iter()
                    .map(|worker| worker.join().expect("contender"))
                    .filter(|owned| *owned)
                    .count(),
                1
            );
        });
        assert_eq!(probe(&root.0), Ok(()));
    }

    #[test]
    #[expect(clippy::panic, reason = "exercises ownership cleanup during unwinding")]
    fn unwinding_releases_ownership_without_deleting_the_lock() {
        let root = Scratch::new();
        let result = std::panic::catch_unwind(|| {
            let _lease = Lease::acquire(&root.0).expect("owner");
            assert_eq!(probe(&root.0), Err(Refusal::Busy));
            panic!("private lease unwind fixture");
        });
        assert!(result.is_err());
        assert_eq!(probe(&root.0), Ok(()));
        assert_eq!(
            fs::metadata(root.0.join(NAME))
                .expect("persistent file")
                .len(),
            0
        );
    }

    #[test]
    fn aliases_resolve_to_one_slot_and_distinct_stores_remain_independent() {
        let root = Scratch::new();
        let other = Scratch::new();
        let alias = other.0.join("alias");
        std::os::unix::fs::symlink(&root.0, &alias).expect("store alias");
        let _lease = Lease::acquire(&root.0).expect("owner");
        assert_eq!(probe(&alias), Err(Refusal::Busy));
        assert!(matches!(Lease::acquire(&alias), Err(Refusal::Busy)));
        assert!(Lease::acquire(&other.0).is_ok());
    }

    #[test]
    fn missing_store_and_nonregular_or_changed_lock_paths_refuse_without_repair() {
        let root = Scratch::new();
        assert!(matches!(
            probe(&root.0.join("missing")),
            Err(Refusal::Unavailable(_))
        ));
        let lock_path = root.0.join(NAME);
        fs::write(&lock_path, b"unexpected bytes").expect("damaged lock fixture");
        assert!(matches!(probe(&root.0), Err(Refusal::Unavailable(_))));
        assert!(matches!(
            Lease::acquire(&root.0),
            Err(Refusal::Unavailable(_))
        ));
        assert_eq!(
            fs::read(&lock_path).expect("unchanged bytes"),
            b"unexpected bytes"
        );
        fs::remove_file(&lock_path).expect("remove private damaged fixture");
        fs::create_dir(&lock_path).expect("directory fixture");
        assert!(matches!(probe(&root.0), Err(Refusal::Unavailable(_))));
        fs::remove_dir(&lock_path).expect("remove private directory");
        let target = root.0.join("target");
        fs::write(&target, b"").expect("target fixture");
        std::os::unix::fs::symlink(&target, &lock_path).expect("symlink fixture");
        assert!(matches!(probe(&root.0), Err(Refusal::Unavailable(_))));
        assert!(matches!(
            Lease::acquire(&root.0),
            Err(Refusal::Unavailable(_))
        ));
        fs::remove_file(&lock_path).expect("remove private symlink");
        fs::hard_link(&target, &lock_path).expect("hardlink fixture");
        assert!(matches!(probe(&root.0), Err(Refusal::Unavailable(_))));
    }
}
