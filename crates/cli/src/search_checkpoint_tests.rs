use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

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
