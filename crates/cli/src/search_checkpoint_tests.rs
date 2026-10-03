#![cfg(test)]
use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Something to run while `publish_inner` holds the payload lock.
type PayloadHook = Box<dyn FnOnce(&File)>;

thread_local! {
    /// Armed by a test on its own thread, taken by the next publication on
    /// that thread and by nothing else.
    static PAYLOAD_LOCKED: RefCell<Option<PayloadHook>> = const { RefCell::new(None) };
    /// Armed by a test, taken by the next publication on this thread once its
    /// `complete` marker is visible to readers.
    static MARKER_VISIBLE: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

/// Called by `publish_inner` right after its marker becomes discoverable.
pub(super) fn marker_visible() {
    if let Some(hook) = MARKER_VISIBLE.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
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

/// CE-3, D-1909: a completion marker cut short by a crash is an interrupted
/// reservation. The newest whole checkpoint is `latest`, and the resume
/// publishes the next sequence. A publication leaves no scratch marker.
#[test]
fn a_torn_completion_marker_is_an_interrupted_reservation() -> Result<(), String> {
    for kept in [0_u64, 31] {
        let scratch = Scratch::new().map_err(error)?;
        let mut journal = Journal::open(&scratch.0, "expression-search-v1", [12; 32])?;
        journal.publish(b"valid old", 1024)?;
        journal.publish(b"torn", 1024)?;
        let newest = journal.directory.join("0000000000000002");
        assert!(!newest.join("complete.writing").exists());
        OpenOptions::new()
            .write(true)
            .open(newest.join("complete"))
            .and_then(|marker| marker.set_len(kept))
            .map_err(error)?;
        drop(journal);
        let mut reopened = Journal::open(&scratch.0, "expression-search-v1", [12; 32])?;
        let latest = reopened
            .latest(1024)?
            .ok_or("the whole checkpoint is latest")?;
        assert_eq!(latest.sequence, 1);
        assert_eq!(latest.payload, b"valid old");
        let (sequence, _) = reopened.publish(b"resumed", 1024)?;
        assert_eq!(sequence, 3);
        let resumed = reopened.latest(1024)?.ok_or("the resume is latest")?;
        assert_eq!(resumed.payload, b"resumed");
    }
    Ok(())
}

/// locks-3, D-1913: once a checkpoint's marker is discoverable, a reader of
/// that checkpoint is never refused as busy by the publishing writer.
#[test]
fn a_discoverable_checkpoint_is_never_refused_as_busy() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", [13; 32])?;
    let seen: Rc<RefCell<Option<Result<Vec<u8>, String>>>> = Rc::default();
    let into = Rc::clone(&seen);
    let base = scratch.0.clone();
    MARKER_VISIBLE.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            let read = Snapshot::open(&base, "and-checkpoint-v1", [13; 32])
                .and_then(|snapshot| snapshot.ok_or_else(|| "namespace".to_owned()))
                .and_then(|snapshot| snapshot.read(1, 1024))
                .map(|saved| saved.payload);
            *into.borrow_mut() = Some(read);
        }));
    });
    journal.publish(b"visible", 1024)?;
    let read = seen
        .borrow_mut()
        .take()
        .ok_or("the hook ran once the marker was visible")?;
    assert_eq!(read?, b"visible");
    Ok(())
}
