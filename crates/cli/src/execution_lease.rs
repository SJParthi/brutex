//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! One nonblocking execution lease per canonical store for cooperating CLI and
//! HTTP sweep entry points. This is admission authority, not a result receipt.
//!
//! A probe opens one fixed path and checks one OS lock. It reads no history and
//! allocates no storage proportional to bars, candidates or previous commands.
//! OS/file latency is not constant-time. Older binaries and callers that bypass
//! the entry points do not participate; their telemetry must remain visible.
//! The empty lock file is never removed or replaced by this protocol.
//!
//! A probe locks the lease file for an instant to read it, and a claimant that
//! met that instant used to be refused as though a sweep owned the store. So
//! both take a second empty file, the probe gate, around their look at the
//! lease: a probe touches the lease only while it holds the gate, so a
//! claimant refused once retries under the gate, where a refusal can only be
//! an owner's. Nobody holds the gate across a run. D-2774.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use store::flock::Flock;

const NAME: &str = ".sweep-execution-v1.lock";
/// Held only around a look at [`NAME`], never across a run (D-2774).
const GATE: &str = ".sweep-execution-v1.probe-gate";

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
    options.custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK);
    options
}

/// Takes the slot without waiting. The guard releases it by an explicit
/// unlock, never by closing the descriptor: a duplicate left in a child
/// another thread spawned would otherwise keep the store busy (D-0693).
fn lock(file: File, path: &Path) -> Result<Flock<File>, Refusal> {
    Flock::try_lock(file, path.to_path_buf()).map_err(|why| match why {
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
/// Dropping unlocks the lease explicitly before its descriptor closes; it never
/// deletes the persistent lock path.
#[derive(Debug)]
#[must_use = "retain the lease throughout computation and terminal audit"]
pub struct Lease {
    _file: Flock<File>,
}

impl Lease {
    /// Atomically claims the one store slot. It never waits for another
    /// writer; it may wait for one probe's look at the slot (D-2774).
    ///
    /// # Errors
    /// Refuses a held lease, an unavailable store, or an unsafe/nonempty lock
    /// path. No command is queued and no existing file is repaired.
    pub fn acquire(root: &Path) -> Result<Self, Refusal> {
        let path = path(root)?;
        // THE GATE EXISTS BEFORE THE LEASE IT GUARDS, so a probe that finds
        // the lease also finds the gate, unless the lease predates the gate.
        let gate_path = path.with_file_name(GATE);
        let gate = create(&gate_path)?;
        match lock(create(&path)?, &path) {
            Ok(held) => {
                verify(&held, &path)?;
                return Ok(Self { _file: held });
            }
            Err(Refusal::Busy) => {}
            Err(why) => return Err(why),
        }
        // REFUSED ONCE: by an owner, or by a probe's instant. A probe holds
        // the gate while it holds the lease, so under the gate a refusal is an
        // owner's (or a non-participating older binary's). The wait for the
        // gate is one probe's open, lock, two stats and unlock. conc:runs-2.
        #[cfg(test)]
        tests::refused_once();
        let gate = Flock::lock(gate, gate_path.clone()).map_err(unavailable)?;
        verify(&gate, &gate_path)?;
        let held = lock(create(&path)?, &path)?;
        verify(&held, &path)?;
        drop(gate);
        Ok(Self { _file: held })
    }
}

/// Opens, creating when absent, one of this protocol's empty lock files.
fn create(path: &Path) -> Result<File, Refusal> {
    options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(unavailable)
}

/// Checks current admission without creating a file or changing a result.
/// Availability can change immediately; every POST must acquire its own lease.
///
/// # Errors
/// Refuses a held lease, an unavailable store, or an unsafe/nonempty lock
/// path. A missing lock file in an existing store is an available snapshot.
///
/// The look at the lease is made under the probe gate, so a claimant that met
/// it retries under the gate rather than being refused (D-2774). A lease left
/// by a binary that predates the gate is looked at without it.
pub fn probe(root: &Path) -> Result<(), Refusal> {
    let path = path(root)?;
    let file = match options().read(true).open(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(why) => return Err(unavailable(why)),
    };
    let gate_path = path.with_file_name(GATE);
    let gate = match options().read(true).open(&gate_path) {
        Ok(gate) => {
            let gate = Flock::lock(gate, gate_path.clone()).map_err(unavailable)?;
            verify(&gate, &gate_path)?;
            Some(gate)
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => return Err(unavailable(why)),
    };
    let held = lock(file, &path)?;
    verify(&held, &path)?;
    held.release().map_err(unavailable)?;
    drop(gate);
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "private concurrency fixtures must fail loudly"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    thread_local! {
        /// Run once when a claimant is refused on its first, ungated attempt.
        static ON_REFUSED_ONCE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
            const { std::cell::RefCell::new(None) };
    }

    pub(super) fn refused_once() {
        if let Some(hook) = ON_REFUSED_ONCE.with(|slot| slot.borrow_mut().take()) {
            hook();
        }
    }

    /// **A probe's look at the lease does not refuse a claimant.**
    /// conc:runs-2, D-2774.
    ///
    /// A status GET probes by locking the lease file for an instant, and a
    /// claim that met that instant was refused as "another sweep owns this
    /// store" with no sweep running. The probe is modelled mid-look, holding
    /// the gate and the lease as `probe` does; it finishes when the claimant
    /// has been refused once, and the claimant must then own the slot.
    #[test]
    fn a_claim_that_meets_a_probe_mid_look_owns_the_slot_once_the_probe_ends() {
        let root = Scratch::new();
        drop(Lease::acquire(&root.0).expect("the lock files exist"));
        let lease_path = root.0.join(NAME);
        let gate_path = root.0.join(GATE);
        let probe_gate = Flock::lock(
            options().read(true).open(&gate_path).expect("the gate"),
            gate_path,
        )
        .expect("the probe takes the gate");
        let probe_look = lock(
            options().read(true).open(&lease_path).expect("the lease"),
            &lease_path,
        )
        .expect("the probe locks the lease for its look");
        ON_REFUSED_ONCE.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move || {
                probe_look.release().expect("the probe's unlock");
                drop(probe_gate);
            }));
        });
        let owned = Lease::acquire(&root.0);
        ON_REFUSED_ONCE.with(|slot| slot.borrow_mut().take());
        let owned = owned.expect("a probe's instant is not an owner");
        assert!(
            matches!(Lease::acquire(&root.0), Err(Refusal::Busy)),
            "and a real owner still refuses the next claimant, under the gate too"
        );
        assert_eq!(probe(&root.0), Err(Refusal::Busy));
        drop(owned);
        assert_eq!(probe(&root.0), Ok(()));
    }

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

    /// A dropped lease frees the store while a duplicate of its descriptor is
    /// still open — the reference a child spawned by another thread holds
    /// until its exec. While closing the descriptor was the release, that
    /// reference kept the slot and every later sweep was `Refusal::Busy` with
    /// no sweep running. D-0693.
    #[test]
    #[expect(
        clippy::used_underscore_binding,
        reason = "the duplicate is taken of the lease's own lock, which is the \
                  field whose drop this test is about"
    )]
    fn a_dropped_lease_is_released_despite_a_duplicated_descriptor() {
        let root = Scratch::new();
        let lease = Lease::acquire(&root.0).expect("first owner");
        let child = lease
            ._file
            .try_clone()
            .expect("the duplicate a spawned child would hold");
        assert_eq!(probe(&root.0), Err(Refusal::Busy), "the premise");
        drop(lease);
        assert_eq!(
            probe(&root.0),
            Ok(()),
            "the dropped lease was released despite the duplicate"
        );
        let again = Lease::acquire(&root.0).expect("and the slot can be claimed again");
        drop(again);
        drop(child);
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
