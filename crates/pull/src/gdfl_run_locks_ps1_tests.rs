//! BASE-COV-PULL-PS1 (D-7335): the lock and claim rows no test had reached at
//! tip 2e7a0b6d3b56 (Route B, recipe v4). A census that cannot be located and
//! one that changed under the locks; a lock name with no folder and a lock
//! path with no file name; a file and a path on two devices; a lock file
//! replaced after it was opened; a descriptor the host will not flock; a
//! process with one descriptor left, and a busy lock whose retry finds none;
//! a release that holds no store lock and one whose claim will not clear; a
//! probe of a dead holder's lock; an executable whose name is gone; and a
//! sidecar whose read stops short of its measured length. Every claim, host
//! and path is invented.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

/// The store lock of `store`, taken exclusive at once.
fn store_lock(store: &Path) -> Result<Flock<File>, LockFault> {
    take(
        open_lock_file(store, STORE_LOCK)?,
        store,
        STORE_LOCK,
        Hold::Exclusive,
        0,
    )
}

/// The most descriptors a fill takes before it says the host's limit is past
/// what a test should hold.
const DESCRIPTOR_BOUND: usize = 1 << 22;

/// Every descriptor this process may still open, held, and the host's refusal
/// of one more.
fn every_descriptor() -> (Vec<File>, io::Error) {
    let seed = File::open("/dev/null").unwrap();
    let mut held = Vec::new();
    let refused = loop {
        match seed.try_clone() {
            Ok(one) => held.push(one),
            Err(why) => break why,
        }
        assert!(
            held.len() < DESCRIPTOR_BOUND,
            "a descriptor limit past {DESCRIPTOR_BOUND}"
        );
    };
    held.push(seed);
    (held, refused)
}

// ── the census under the locks ──────────────────────────────────────────────

/// A store whose version-4 census name holds a file is no census either
/// layout reads: the layout is refused `census_unreadable` in the census's
/// words, a run is refused before it opens a lock, and locks already held
/// when the second look fails are every one released.
#[test]
fn a_census_that_cannot_be_located_is_refused_and_frees_every_lock() {
    let store = scratch("ps1-locks-census-file");
    fs::create_dir_all(store.join("manifest/v4")).unwrap();
    fs::write(store.join("manifest/v4/gdfl"), b"not a census directory").unwrap();
    let Err(why) = locate(&store, Vendor::Gdfl) else {
        panic!("a file at the census's name was located");
    };
    let unreadable = LockFault::Layout { why };
    assert_eq!(layout_at(&store), Err(unreadable.clone()));
    assert_eq!(
        RunLocks::take(&store, "stocks", 0).err(),
        Some(unreadable.clone())
    );
    assert!(!store.join(STORE_LOCK).exists(), "no lock file was opened");
    let held = store_lock(&store).unwrap();
    let refused = RunLocks::held(
        &store,
        (GdflLayout::V3, true),
        (Some(held), None),
        STORE_LOCK.to_owned(),
    );
    assert_eq!(refused.err(), Some(unreadable));
    assert!(
        store_lock(&store).is_ok(),
        "the lock the refusal held is free"
    );
}

/// Locks taken for v3 on a store that is v4 when they are checked are refused
/// `census_layout_changed`, naming both layouts, and released.
#[test]
fn locks_taken_for_another_layout_are_refused_and_released() {
    let store = scratch("ps1-locks-layout");
    fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let held = store_lock(&store).unwrap();
    let refused = RunLocks::held(
        &store,
        (GdflLayout::V3, true),
        (Some(held), None),
        STORE_LOCK.to_owned(),
    );
    assert_eq!(
        refused.err(),
        Some(LockFault::LayoutChanged {
            locked: GdflLayout::V3,
            found: GdflLayout::V4,
        })
    );
    assert!(
        store_lock(&store).is_ok(),
        "the lock the refusal held is free"
    );
}

// ── the release ─────────────────────────────────────────────────────────────

/// Locks that hold a kind lock and no store lock own no claim: they read no
/// dead holder's claim, their release clears nothing and leaves the bytes
/// another holder wrote at the claim's name as they are, and it answers that
/// all went well.
#[test]
fn locks_that_hold_no_store_lock_own_no_claim_and_clear_none() {
    let store = scratch("ps1-locks-no-store-lock");
    let name = kind_lock("stocks");
    let kind = take(
        open_lock_file(&store, &name).unwrap(),
        &store,
        &name,
        Hold::Exclusive,
        0,
    )
    .unwrap();
    let claim = b"claim of an invented other holder\n";
    fs::write(store.join(&name), claim).unwrap();
    let locks = RunLocks::held(
        &store,
        (GdflLayout::V4, false),
        (None, Some(kind)),
        name.clone(),
    )
    .unwrap();
    assert_eq!(
        locks.dead(),
        None,
        "a claim under no store lock is not its own"
    );
    assert_eq!(locks.release(), Ok(()));
    assert_eq!(fs::read(store.join(&name)).unwrap(), claim);
}

/// A claim lock held through a descriptor that may read and not write cannot
/// be cleared: the release names the claim and the host's reason, and frees
/// the lock all the same.
#[test]
fn a_claim_that_will_not_clear_is_named_and_its_lock_still_freed() {
    let store = scratch("ps1-locks-read-only");
    drop(open_lock_file(&store, STORE_LOCK).unwrap());
    let path = store.join(STORE_LOCK);
    let read_only = Flock::try_lock(File::open(&path).unwrap(), path.clone()).unwrap();
    let locks = RunLocks::held(
        &store,
        (GdflLayout::V3, false),
        (Some(read_only), None),
        STORE_LOCK.to_owned(),
    )
    .unwrap();
    let host = File::open(&path).unwrap().set_len(0).unwrap_err();
    assert_eq!(
        locks.release(),
        Err(format!("{STORE_LOCK}: the claim was not cleared ({host})"))
    );
    assert!(store_lock(&store).is_ok(), "the lock is free");
}

// ── taking a lock ───────────────────────────────────────────────────────────

/// A lock file removed and made again after it was opened is not the file its
/// name holds: a take of the old handle is refused `lock_replaced`, by the
/// counted loop and by the timed one alike, and the new file stays free.
#[test]
fn a_lock_file_replaced_after_it_was_opened_is_refused_as_replaced() {
    let store = scratch("ps1-locks-replaced");
    let old = open_lock_file(&store, STORE_LOCK).unwrap();
    let kept = old.try_clone().unwrap();
    let path = store.join(STORE_LOCK);
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"").unwrap();
    let replaced = LockFault::Replaced {
        lock: STORE_LOCK.to_owned(),
    };
    assert_eq!(
        take(old, &store, STORE_LOCK, Hold::Exclusive, 0).err(),
        Some(replaced.clone())
    );
    assert_eq!(
        hold_within(kept, &path, STORE_LOCK, Duration::ZERO).err(),
        Some(replaced)
    );
    assert!(
        store_lock(&store).is_ok(),
        "the file at the name was never locked"
    );
}

/// A lock name with no folder above it (the root itself) makes no folder and
/// is refused for what stands there.
#[test]
fn a_lock_name_with_no_folder_above_it_is_refused_for_what_stands_there() {
    let store = scratch("ps1-locks-root-name");
    let Err(LockFault::Refused { lock, why }) = open_lock_file(&store, "/") else {
        panic!("the root opened as a lock file");
    };
    assert_eq!(lock, "/");
    assert!(
        why.starts_with("is a directory and cannot be opened ("),
        "{why}"
    );
}

/// A lock path with no file name of its own (`<folder>/..`) is named by the
/// whole path, and refused for the directory it is.
#[test]
fn a_lock_path_with_no_file_name_is_named_by_its_whole_path() {
    let store = scratch("ps1-locks-dotdot");
    let path = store.join("..");
    assert_eq!(path.file_name(), None);
    let Err(LockFault::Refused { lock, why }) = take_path_within(&path, Duration::ZERO) else {
        panic!("a directory opened as a lock file");
    };
    assert_eq!(lock, path.display().to_string());
    assert!(
        why.starts_with("is a directory and cannot be opened ("),
        "{why}"
    );
}

/// A file and a path on two devices are never one file, even where their
/// inode numbers agree (Linux's procfs and sysfs roots are both inode 1).
#[test]
fn a_file_and_a_path_on_two_devices_are_never_one_file() {
    let store = scratch("ps1-locks-devices");
    let here = store.join("a");
    fs::write(&here, b"x").unwrap();
    assert!(!same_file(
        &File::open(&here).unwrap(),
        Path::new("/dev/null")
    ));
    #[cfg(target_os = "linux")]
    {
        let proc_root = File::open("/proc").unwrap();
        assert!(!same_file(&proc_root, Path::new("/sys")));
    }
}

/// A descriptor naming `path` on which the host refuses every flock: Linux's
/// `O_PATH` (`include/uapi/asm-generic/fcntl.h`: `O_PATH 010000000`, which
/// neither `x86_64` nor arm64 overrides), which no lock call accepts.
#[cfg(target_os = "linux")]
fn unlockable(path: &Path) -> File {
    use std::os::unix::fs::OpenOptionsExt as _;
    const O_PATH: i32 = 0o10_000_000;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_PATH)
        .open(path)
        .unwrap()
}

/// A descriptor the host will not flock is refused `run_lock_refused` in the
/// host's own words, at once, by the counted loop (however many retries it
/// was given) and by the timed one (however long it may wait); never read as
/// busy.
#[cfg(target_os = "linux")]
#[test]
fn a_descriptor_the_host_will_not_flock_is_refused_in_its_words() {
    let store = scratch("ps1-locks-unlockable");
    let name = "imports/ps1.lock";
    let path = store.join(name);
    fs::create_dir_all(store.join("imports")).unwrap();
    fs::write(&path, b"").unwrap();
    let Err(TryLockError::Error(host)) = unlockable(&path).try_lock() else {
        panic!("the host took a flock it refuses");
    };
    let words = format!("flock: {host}");
    assert_eq!(
        take(unlockable(&path), &store, name, Hold::Exclusive, 3).err(),
        Some(LockFault::Refused {
            lock: name.to_owned(),
            why: words.clone(),
        })
    );
    assert_eq!(
        hold_within(unlockable(&path), &path, "ps1.lock", LOCK_WAIT).err(),
        Some(LockFault::Refused {
            lock: "ps1.lock".to_owned(),
            why: words,
        })
    );
}

/// A process with one descriptor left probes nothing and takes nothing: the
/// probe's open takes that descriptor and the lock's copy of it finds none,
/// so the probe says unprobed and why; the timed take's open takes it again
/// and its copy finds none. Run in a process of its own (the crate's
/// `where_permission_binds` child), whose descriptors are this test's alone.
#[test]
fn a_process_with_one_descriptor_left_probes_and_takes_nothing() {
    crate::support::where_permission_binds(
        "gdfl_run::locks::ps1_tests::a_process_with_one_descriptor_left_probes_and_takes_nothing",
        a_process_with_one_descriptor_left_probes_and_takes_nothing_body,
    );
}

fn a_process_with_one_descriptor_left_probes_and_takes_nothing_body() {
    let store = scratch("ps1-locks-no-descriptor");
    fs::create_dir_all(store.join("imports")).unwrap();
    fs::write(store.join(STORE_LOCK), b"").unwrap();
    let journal = store.join("imports").join("ps1.journal.lock");
    fs::write(&journal, b"").unwrap();
    let (mut held, full) = every_descriptor();
    drop(held.pop());
    let probed = probe_one(&store, STORE_LOCK, 0);
    let within = take_path_within(&journal, LOCK_WAIT).err();
    drop(held);
    assert_eq!(
        probed,
        LockState::Unprobed {
            lock: STORE_LOCK.to_owned(),
            why: format!("run_lock_refused: {STORE_LOCK}: {full}"),
        }
    );
    assert_eq!(
        within,
        Some(LockFault::Refused {
            lock: "ps1.journal.lock".to_owned(),
            why: full.to_string(),
        })
    );
}

// ── the probe, the executable, the sidecar ─────────────────────────────────

/// A lock no one holds whose file still holds bytes was left by a holder that
/// died: the probe says interrupted, under the lock's own name.
#[test]
fn a_lock_left_holding_bytes_probes_as_interrupted_under_its_name() {
    let store = scratch("ps1-locks-interrupted");
    fs::create_dir_all(store.join("imports")).unwrap();
    fs::write(store.join(STORE_LOCK), b"claim of an invented dead run\n").unwrap();
    let state = probe_one(&store, STORE_LOCK, 0);
    assert!(
        matches!(
            &state,
            LockState::Interrupted {
                claim: ClaimState::Unreadable(_),
                stop_requested: false,
                ..
            }
        ),
        "{state:?}"
    );
    assert_eq!(state.lock(), STORE_LOCK);
}

/// The marker under which a linked copy of this test binary runs the child
/// half of `an_executable_whose_name_is_gone_has_no_identity`.
const NAME_GONE_CHILD: &str = "BRUTEX_PS1_EXE_NAME_GONE_CHILD";

/// An executable whose name was removed while it runs has no identity to
/// claim: `exe=` is `-`, never the identity of whatever the name holds next.
/// The child half runs from a hard link of this test binary (a copy where no
/// link can be made) and removes that name first; the binary this process
/// runs from keeps its name and its identity.
#[test]
fn an_executable_whose_name_is_gone_has_no_identity() {
    if std::env::var_os(NAME_GONE_CHILD).is_some() {
        let own = std::env::current_exe().unwrap();
        assert_ne!(exe_field(), "-", "a name that stands has an identity");
        fs::remove_file(&own).unwrap();
        assert_eq!(exe_field(), "-");
        return;
    }
    let dir = scratch("ps1-locks-exe-gone");
    let linked = dir.join("ps1-exe-gone");
    let own = std::env::current_exe().unwrap();
    fs::hard_link(&own, &linked)
        .or_else(|_| fs::copy(&own, &linked).map(drop))
        .unwrap();
    let output = std::process::Command::new(&linked)
        .args([
            "--exact",
            "gdfl_run::locks::ps1_tests::an_executable_whose_name_is_gone_has_no_identity",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(NAME_GONE_CHILD, "1")
        .output()
        .unwrap();
    let said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success() && said.contains("1 passed"),
        "{said}"
    );
    assert!(!linked.exists(), "the child removed its own name");
    assert_ne!(exe_field(), "-");
}

/// A sidecar whose read stops short of the length its open measured is
/// unreadable, never half-read: Linux's sysfs states a whole page for an
/// attribute of a few bytes, the shape of a sidecar cut between the open and
/// the read.
#[cfg(target_os = "linux")]
#[test]
fn a_sidecar_shorter_than_its_measured_length_is_unreadable() {
    let path = Path::new("/sys/devices/system/cpu/online");
    let stated = fs::metadata(path).unwrap().len();
    let read = u64::try_from(fs::read(path).unwrap().len()).unwrap();
    assert_ne!(read, 0, "the attribute holds bytes");
    assert!(read < stated, "{read} bytes of a stated {stated}");
    assert!(
        stated <= u64::try_from(SIDECAR_MAX).unwrap(),
        "a stated {stated} bytes, past the sidecar's bound"
    );
    assert_eq!(
        read_sidecar(path, &digest16(b"an invented request line")),
        SidecarState::Stale("unreadable".to_owned())
    );
}

/// **A RETRY WHOSE COPY OF THE LOCK FILE THE HOST REFUSES IS REFUSED IN THE
/// HOST'S WORDS** (the copy seam, D-7335): with the lock held on another
/// descriptor, a take that may retry once copies the file twice (once to read
/// a holder by, once for the retry); the second copy refused is
/// `run_lock_refused` with the host's words, after the one wait, never
/// `run_busy`. Extremes at once: a retry of exactly one, and the first copy
/// granted beside the second refused.
#[test]
fn a_retry_whose_copy_of_the_lock_the_host_refuses_is_refused_in_its_words() {
    let store = scratch("ps1-locks-copy");
    fs::create_dir_all(store.join("imports")).unwrap();
    let name = "imports/ps1.lock";
    let held = take(
        open_lock_file(&store, name).unwrap(),
        &store,
        name,
        Hold::Exclusive,
        0,
    )
    .unwrap();
    let copies = std::cell::Cell::new(0_u32);
    let copy = |file: &File| {
        copies.set(copies.get() + 1);
        if copies.get() == 1 {
            file.try_clone()
        } else {
            Err(std::io::Error::other("an invented refusal"))
        }
    };
    let refused = take_cloning(
        open_lock_file(&store, name).unwrap(),
        &store,
        name,
        (Hold::Exclusive, 1),
        &copy,
    )
    .unwrap_err();
    assert_eq!(
        refused,
        LockFault::Refused {
            lock: name.to_owned(),
            why: "an invented refusal".to_owned(),
        }
    );
    assert_eq!(copies.get(), 2);
    held.release().unwrap();
}

/// **AN UNLOCK THE HOST REFUSES IS NAMED FOR EACH LOCK, AND EVERY LOCK IS
/// STILL TRIED** (the release seam, D-7335): the kind lock and the store lock
/// of one run both refused are two faults in the host's words, and the run's
/// release after them has nothing left to free. Extremes at once: every
/// unlock refused, then a release of nothing.
#[test]
fn an_unlock_the_host_refuses_is_named_for_each_lock() {
    let store = scratch("ps1-locks-unlock");
    fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let mut locks = RunLocks::take(&store, "indices", 0).unwrap();
    let mut faults = Vec::new();
    locks.unlock_into_with(&mut faults, &|flock: Flock<File>| {
        flock.release()?;
        Err(store::flock::Unreleased {
            path: PathBuf::from("/invented/lock"),
            why: std::io::Error::other("an invented refusal"),
        })
    });
    assert_eq!(faults.len(), 2, "{faults:?}");
    assert!(
        faults.iter().all(|fault| fault
            .starts_with("the advisory lock on /invented/lock could not be released")
            && fault.ends_with(": an invented refusal")),
        "{faults:?}"
    );
    locks.release().unwrap();
}
