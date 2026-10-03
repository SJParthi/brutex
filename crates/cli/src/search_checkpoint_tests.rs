#![cfg(test)]
use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Something to run while `publish_inner` holds the payload lock.
type PayloadHook = Box<dyn FnOnce(&File)>;

thread_local! {
    /// A lowered `DIRECTORY_LIMIT` for this test thread only.
    pub(super) static LIMIT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Armed by a test on its own thread, taken by the next publication on
    /// that thread and by nothing else.
    static PAYLOAD_LOCKED: RefCell<Option<PayloadHook>> = const { RefCell::new(None) };
}

thread_local! {
    /// Armed by a test: the next staged marker on this thread fails here, after
    /// its seal is synced under `complete.tmp` and before the rename.
    static MARKER_FAULT: std::cell::Cell<Option<std::io::ErrorKind>> = const { std::cell::Cell::new(None) };
}

/// Called by `publish_marker` between the synced temporary marker and its
/// rename; fails once when a test armed it.
pub(super) fn marker_staged(_temporary: &Path) -> std::io::Result<()> {
    MARKER_FAULT
        .with(std::cell::Cell::take)
        .map_or(Ok(()), |kind| {
            Err(std::io::Error::new(kind, "injected rename failure"))
        })
}

/// Called by `publish_inner` right after it takes the payload lock: runs the
/// hook a test armed, once. The regression below uses it to take the
/// duplicate descriptor a child spawned at that moment would hold.
pub(super) fn payload_locked(file: &File) {
    if let Some(hook) = PAYLOAD_LOCKED.with(|slot| slot.borrow_mut().take()) {
        hook(file);
    }
}

pub(crate) struct Scratch(pub PathBuf);
impl Scratch {
    pub(crate) fn new() -> std::io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "brutex-search-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn restart_skips_uncommitted_reservations_without_reusing_or_losing_completed_bytes()
-> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [1; 32])?;
    assert!(Journal::open(&scratch.0, "and-checkpoint-v1", [1; 32]).is_err());
    assert!(journal.latest(1024)?.is_none());
    let first = journal.publish(b"first", 1024)?;
    let second = journal.publish(b"second", 1024)?;
    assert_eq!((first.0, second.0), (1, 2));
    let original = fs::read(journal.directory.join("0000000000000001/payload")).map_err(error)?;
    fs::create_dir(journal.directory.join("0000000000000003")).map_err(error)?;
    drop(journal);
    let mut reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [1; 32])?;
    assert_eq!(reopened.interrupted(), 1);
    assert_eq!(reopened.next_sequence(), 4);
    assert_eq!(
        reopened.latest(1024)?.ok_or("missing latest")?.payload,
        b"second"
    );
    assert_eq!(reopened.publish(b"fourth", 1024)?.0, 4);
    assert_eq!(reopened.next_sequence(), 5);
    assert_eq!(
        fs::read(reopened.directory.join("0000000000000001/payload")).map_err(error)?,
        original
    );
    assert_eq!(reopened.read(1, 1024)?.seal, first.1);
    assert!(reopened.read(3, 1024).is_err());
    Ok(())
}

#[test]
fn latest_corruption_never_falls_back_to_an_older_valid_checkpoint() -> Result<(), String> {
    for target in ["payload", "complete"] {
        let scratch = Scratch::new().map_err(error)?;
        let mut journal = Journal::open(&scratch.0, "expression-search-v1", [2; 32])?;
        journal.publish(b"valid old", 1024)?;
        journal.publish(b"new", 1024)?;
        let path = journal.directory.join("0000000000000002").join(target);
        let mut bytes = fs::read(&path).map_err(error)?;
        *bytes.last_mut().ok_or("empty fixture")? ^= 1;
        fs::write(path, bytes).map_err(error)?;
        assert!(journal.latest(1024).is_err());
        drop(journal);
        let reopened = Journal::open(&scratch.0, "expression-search-v1", [2; 32])?;
        assert!(reopened.latest(1024).is_err());
        assert_eq!(reopened.read(1, 1024)?.payload, b"valid old");
    }
    Ok(())
}

#[test]
fn admission_foreign_identity_and_poisoned_writer_refuse_explicitly() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    assert!(Journal::open(&scratch.0, "../escape", [3; 32]).is_err());
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [3; 32])?;
    journal.publish(b"abc", 99)?;
    assert!(journal.latest(98).is_err());
    assert!(journal.publish(b"abcd", 99).is_err());
    assert!(journal.publish(b"", 1024).is_err());
    let foreign = Journal::open(&scratch.0, "and-checkpoint-v1", [4; 32])?;
    assert!(foreign.latest(1024)?.is_none());
    let source = journal.directory.join("0000000000000001");
    let destination = foreign.directory.join("0000000000000001");
    fs::create_dir(&destination).map_err(error)?;
    for name in ["payload", "complete"] {
        fs::copy(source.join(name), destination.join(name)).map_err(error)?;
    }
    assert!(foreign.read(1, 1024).is_err());
    fs::write(journal.directory.join("bad-name"), b"?").map_err(error)?;
    drop(journal);
    assert!(Journal::open(&scratch.0, "and-checkpoint-v1", [3; 32]).is_err());
    Ok(())
}

#[test]
fn acknowledgment_detects_in_place_mutation_and_path_replacement() -> Result<(), String> {
    for replacement in [false, true] {
        let scratch = Scratch::new().map_err(error)?;
        let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [5; 32])?;
        let (sequence, seal) = journal.publish(b"actual", 1024)?;
        let path = journal.directory.join(format!("{sequence:016x}/payload"));
        let mut held = File::open(&path).map_err(error)?;
        let bytes = fs::read(&path).map_err(error)?;
        if replacement {
            let next = path.with_extension("replacement");
            fs::write(&next, bytes).map_err(error)?;
            fs::rename(next, &path).map_err(error)?;
        } else {
            let mut writer = OpenOptions::new().write(true).open(&path).map_err(error)?;
            writer.seek(SeekFrom::Start(64)).map_err(error)?;
            writer.write_all(b"mutant").map_err(error)?;
        }
        assert!(
            verify_acknowledged(
                &mut held,
                &path,
                &header_of([5; 32], sequence, 6),
                b"actual",
                seal
            )
            .is_err()
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn dangling_owner_symlink_refuses_without_creating_a_foreign_file() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let journal = Journal::open(&scratch.0, "and-checkpoint-v1", [7; 32])?;
    let owner = journal.directory.join("owner.lock");
    drop(journal);
    fs::remove_file(&owner).map_err(error)?;
    let foreign = scratch.0.join("foreign");
    std::os::unix::fs::symlink(&foreign, &owner).map_err(error)?;
    assert!(Journal::open(&scratch.0, "and-checkpoint-v1", [7; 32]).is_err());
    assert!(
        !foreign.exists(),
        "refusing a symlink must not create its target"
    );
    Ok(())
}

/// THE CI FAILURE, DETERMINISTICALLY: a published checkpoint is readable while
/// a duplicate of its payload descriptor is still open. D-0693.
///
/// The duplicate is taken while the payload lock is held, which is exactly
/// what a child spawned by another thread at that moment inherits until its
/// exec. While closing the descriptor was the release, that reference kept the
/// exclusive lock alive and `latest` refused with "checkpoint payload is busy
/// or cannot be read: ... would block".
#[test]
fn a_published_checkpoint_is_released_despite_a_duplicated_descriptor() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [9; 32])?;
    let captured: Rc<RefCell<Option<File>>> = Rc::default();
    let into = Rc::clone(&captured);
    PAYLOAD_LOCKED.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |file: &File| {
            *into.borrow_mut() = file.try_clone().ok();
        }));
    });
    let (sequence, seal) = journal.publish(b"survives a child", 1024)?;
    let child = captured
        .borrow_mut()
        .take()
        .ok_or("the hook ran under the payload lock and duplicated it")?;

    let saved = journal
        .latest(1024)?
        .ok_or("the acknowledged checkpoint is the latest")?;
    assert_eq!(saved.payload, b"survives a child");
    assert_eq!((saved.sequence, saved.seal), (sequence, seal));
    let snapshot =
        Snapshot::open(&scratch.0, "and-checkpoint-v1", [9; 32])?.ok_or("the namespace exists")?;
    assert_eq!(
        snapshot.read(sequence, 1024)?.payload,
        b"survives a child",
        "a read-only observer reads it too"
    );
    drop(child);
    Ok(())
}

/// A dropped journal releases its namespace while a duplicate of the owner
/// descriptor is still open. D-0693.
#[test]
fn a_dropped_search_journal_is_released_despite_a_duplicated_descriptor() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let journal = Journal::open(&scratch.0, "and-checkpoint-v1", [10; 32])?;
    let child = journal.owner.try_clone().map_err(error)?;
    assert!(
        Journal::open(&scratch.0, "and-checkpoint-v1", [10; 32]).is_err(),
        "the premise: a live owner refuses a second one"
    );
    drop(journal);

    let reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [10; 32])?;
    drop(reopened);
    let snapshot =
        Snapshot::open(&scratch.0, "and-checkpoint-v1", [10; 32])?.ok_or("the namespace exists")?;
    assert!(
        !snapshot.writer_observed,
        "no writer is observed once every owner has dropped"
    );
    drop(child);
    Ok(())
}

/// GAP11-0: a kill between creating the completion marker and writing its
/// seal left a 0-byte `complete` that discovery counted as acknowledged and
/// `read` refused forever ("checkpoint marker width mismatch"), so no rerun of
/// that search could ever resume. The state is built with real files: a fully
/// synced payload for reservation 2 plus the empty marker the old protocol
/// left. D-1640.
#[test]
fn an_empty_marker_left_by_a_kill_is_an_interrupted_reservation() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [11; 32])?;
    let (first, seal) = journal.publish(b"first", 1024)?;
    journal.publish(b"second", 1024)?;
    let second = journal.directory.join("0000000000000002");
    drop(journal);
    fs::remove_file(second.join("complete")).map_err(error)?;
    File::create_new(second.join("complete")).map_err(error)?;

    let snapshot =
        Snapshot::open(&scratch.0, "and-checkpoint-v1", [11; 32])?.ok_or("the namespace exists")?;
    assert_eq!(
        (snapshot.latest, snapshot.interrupted, snapshot.acknowledged),
        (Some(1), 1, 1)
    );
    let mut reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [11; 32])?;
    assert_eq!((reopened.interrupted(), reopened.acknowledged()), (1, 1));
    let latest = reopened
        .latest(1024)?
        .ok_or("checkpoint 1 is the resume point")?;
    assert_eq!((latest.sequence, latest.seal), (first, seal));
    assert_eq!(latest.payload, b"first");
    assert!(
        reopened.read(2, 1024).is_err(),
        "the torn one is never read"
    );
    assert_eq!(reopened.publish(b"third", 1024)?.0, 3);
    assert_eq!(reopened.latest(1024)?.ok_or("latest")?.payload, b"third");
    Ok(())
}

/// W2-cli13-5: `publish` acknowledged a reservation that took the namespace
/// past the entry ceiling discovery admits, so every later `Journal::open` and
/// `Snapshot::open` refused the search for good. The ceiling is lowered on this
/// thread so it is reached with real directories: owner.lock plus three
/// reservations is exactly the limit of four. D-1640.
#[test]
fn publishing_never_acknowledges_a_checkpoint_discovery_cannot_reopen() -> Result<(), String> {
    LIMIT.with(|limit| limit.set(Some(4)));
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [12; 32])?;
    journal.publish(b"one", 1024)?;
    drop(journal);
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [12; 32])?;
    journal.publish(b"two", 1024)?;
    let (third, _) = journal.publish(b"three", 1024)?;
    assert_eq!(
        third, 3,
        "limit - 1 reservations plus the owner is the limit"
    );
    let refused = journal.publish(b"four", 1024);
    assert_eq!(
        refused,
        Err("checkpoint namespace reached its directory admission limit".to_owned())
    );
    assert!(
        !journal.directory.join("0000000000000004").exists(),
        "the refusal comes before the reservation is created"
    );
    drop(journal);
    let reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [12; 32])?;
    assert_eq!(reopened.latest(1024)?.ok_or("latest")?.payload, b"three");
    let mut reopened = reopened;
    assert!(
        reopened.publish(b"four", 1024).is_err(),
        "a reopen counts the same entries"
    );
    drop(reopened);
    fs::create_dir(
        scratch
            .0
            .join("and-checkpoint-v1")
            .join(hex(&[12; 32]))
            .join("0000000000000009"),
    )
    .map_err(error)?;
    assert_eq!(
        Journal::open(&scratch.0, "and-checkpoint-v1", [12; 32]).err(),
        Some("checkpoint directory admission limit exceeded".to_owned()),
        "one entry past the limit is what discovery refuses"
    );
    LIMIT.with(|limit| limit.set(None));
    Ok(())
}

/// The new protocol's own crash window: a kill after the seal is synced under
/// `complete.tmp` and before the rename leaves that file, which discovery
/// never reads, so the reservation is interrupted and the previous checkpoint
/// stays the resume point. A failure the writer sees there (ENOSPC at the
/// rename) removes the temporary file, refuses, and acknowledges nothing.
#[test]
fn a_staged_marker_is_never_an_acknowledgment() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [13; 32])?;
    journal.publish(b"first", 1024)?;
    MARKER_FAULT.with(|fault| fault.set(Some(std::io::ErrorKind::StorageFull)));
    let why = journal
        .publish(b"second", 1024)
        .err()
        .ok_or("the rename refuses")?;
    assert!(
        why.starts_with("checkpoint marker was not published: injected rename failure"),
        "{why}"
    );
    let second = journal.directory.join("0000000000000002");
    assert!(
        !second.join("complete.tmp").exists(),
        "the temporary marker was removed"
    );
    assert!(!second.join("complete").exists());
    assert!(
        journal.publish(b"third", 1024).is_err(),
        "the writer is poisoned"
    );
    drop(journal);

    // A kill at the same point: a whole, synced seal under the temporary name.
    let seal = fs::read(journal_dir(&scratch.0, [13; 32]).join("0000000000000001/complete"))
        .map_err(error)?;
    fs::write(second.join("complete.tmp"), &seal).map_err(error)?;
    let mut reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [13; 32])?;
    assert_eq!((reopened.interrupted(), reopened.acknowledged()), (1, 1));
    assert_eq!(reopened.latest(1024)?.ok_or("latest")?.payload, b"first");
    assert_eq!(reopened.publish(b"third", 1024)?.0, 3);
    let marker = reopened.directory.join("0000000000000003");
    assert!(
        !marker.join("complete.tmp").exists(),
        "a published marker leaves no temporary"
    );
    assert_eq!(fs::read(marker.join("complete")).map_err(error)?.len(), 32);
    Ok(())
}

/// A marker of any other short width is not the empty one a kill left and
/// still refuses loudly when read.
#[test]
fn a_short_nonempty_marker_still_refuses() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [14; 32])?;
    journal.publish(b"only", 1024)?;
    let marker = journal.directory.join("0000000000000001/complete");
    fs::write(&marker, [7_u8; 31]).map_err(error)?;
    drop(journal);
    let reopened = Journal::open(&scratch.0, "and-checkpoint-v1", [14; 32])?;
    assert_eq!((reopened.interrupted(), reopened.acknowledged()), (0, 1));
    assert_eq!(
        reopened.latest(1024).err(),
        Some("checkpoint marker width mismatch".to_owned())
    );
    Ok(())
}

fn journal_dir(root: &Path, identity: [u8; 32]) -> PathBuf {
    root.join("and-checkpoint-v1").join(hex(&identity))
}
