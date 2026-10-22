//! No census file is written through a link planted at its staging name, and
//! no epoch's name is followed (AG-05, assurance finding store-1; D-4922).
//!
//! The epoch install opened `<epoch>.man.tmp` with `File::create` — create,
//! TRUNCATE, following links — so a hard link planted there cut the OLDER
//! epoch's inode and wrote the new image through it (the withdraw answered
//! `Ok`, and two epochs then shared one inode), and a symbolic link sent the
//! image into whatever it named. Every test here fails on that code and
//! passes on `store::staging::create`.
//!
//! Every fixture is invented: keys, counts and stamps, in scratch stores of
//! the tests' own under the host's temporary directory.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use super::{
    FEATURE_CENSUS_V4, Mode, REQUIRES_FORMAT, SEAL_RECEIPT, Target, WholeStep, current_epoch,
    cycle, install_epoch_with, install_run, migrate, read_epoch, record, require_feature,
    requires_path, seal_legacy, superseded_path, withdraw,
};
use crate::manifest::{
    Closes, Entry, EntryKey, Held, Manifest, epoch_path, manifest_path, segment_dir, vendor_dir,
};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-census-link-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).expect("a scratch store");
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn key(n: u32) -> EntryKey {
    EntryKey {
        contract: Contract::parse(&format!("2019-12-26-{}-CE", 1_000_000 + n * 100)),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2019, 11).expect("a month"),
    }
}

fn row(n: u32, rows: u64) -> Held {
    Held::new(
        Entry {
            key: key(n),
            rows,
            first_ts_micros: 10,
            last_ts_micros: 10 + i64::try_from(rows).expect("rows"),
        },
        Closes::UNKNOWN,
    )
}

/// Epoch 1 of the gdfl FNO partition at `capacity`, holding `keys` distinct
/// keys, written as a fixture; returns its path.
fn epoch_one(root: &Path, capacity: u64, keys: u32) -> PathBuf {
    let mut census =
        Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
    for n in 0..keys {
        census.record_held(row(n, u64::from(n) + 1)).expect("held");
    }
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
    std::fs::write(&path, census.image()).expect("epoch 1");
    path
}

/// The staging name of epoch 2.
fn tmp_two(root: &Path) -> PathBuf {
    segment_dir(root, Vendor::Gdfl, Segment::Fno).join("000002.man.tmp")
}

/// `(device, inode, links, bytes)` of `path`, without following it.
fn inode(path: &Path) -> (u64, u64, u64, Vec<u8>) {
    let meta = std::fs::symlink_metadata(path).expect("measured");
    (
        meta.dev(),
        meta.ino(),
        meta.nlink(),
        std::fs::read(path).expect("read"),
    )
}

fn withdraw_key0(root: &Path) -> Result<Option<super::Compaction>, String> {
    withdraw(root, Vendor::Gdfl, Segment::Fno, &|k: &EntryKey| {
        *k == key(0)
    })
}

/// `call`'s answer, taken on a thread of its own: a HANG DETECTOR, not a
/// cost bound (AG-05 repair 1). A call that opened a pipe for writing would
/// wait for a reader forever, and a clock read after such a call never runs,
/// so the old `elapsed() < 5 s` check could only ever fail on a slow host,
/// never on the hang it named. Two minutes is over 190 times the slowest
/// whole fsync'd install this shared host measured under load (617 ms), so
/// only a call that waits on the pipe meets it.
fn answered_without_waiting<T: Send + 'static>(call: impl FnOnce() -> T + Send + 'static) -> T {
    let (tell, told) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tell.send(call());
    });
    told.recv_timeout(std::time::Duration::from_mins(2))
        .expect("answered without waiting on the pipe")
}

/// Epoch 2 was not installed, and the listing still answers epoch 1.
fn nothing_installed(root: &Path) {
    let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    assert!(
        std::fs::symlink_metadata(&two).is_err(),
        "no epoch 2 was installed"
    );
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(1)));
}

/// **A HARD LINK AT THE STAGING NAME IS REFUSED BY NAME, AND THE OLDER EPOCH
/// KEEPS ITS BYTES AND ITS INODE.** Before the fix the withdraw answered `Ok`
/// after cutting epoch 1 to the new image, leaving two epochs on one inode.
#[test]
fn an_epoch_install_never_writes_through_a_hard_link_at_its_staging_name() {
    let scratch = Scratch::new("HARD");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    std::fs::hard_link(&one, tmp_two(root)).expect("a planted hard link");
    let before = inode(&one);
    assert_eq!(before.2, 2, "the fixture: two names of epoch 1");

    let why = withdraw_key0(root).expect_err("refused");
    assert!(why.contains("staging_linked"), "{why}");
    assert!(why.contains(&tmp_two(root).display().to_string()), "{why}");
    assert!(why.contains("one of 2 names"), "{why}");
    assert_eq!(inode(&one), before, "epoch 1: same inode, links and bytes");
    assert_eq!(
        read_epoch(&one, Vendor::Gdfl, Segment::Fno)
            .expect("epoch 1 reads")
            .entries(),
        4,
        "epoch 1 still holds its four entries"
    );
    assert_eq!(
        inode(&tmp_two(root)).1,
        before.1,
        "the planted link is left as it is, for a person to look at"
    );
    nothing_installed(root);
}

/// **A SYMBOLIC LINK AT THE STAGING NAME IS REFUSED, AND THE FILE IT NAMES
/// IS NOT WRITTEN** — a live target, a dangling one, and a link to the older
/// epoch itself. Before the fix each target received the census image and
/// epoch 2 became a symbolic link.
#[test]
fn an_epoch_install_never_writes_through_a_symlink_at_its_staging_name() {
    let scratch = Scratch::new("SYM");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let outside = root.join("outside.txt");
    std::fs::write(&outside, b"an invented file outside the census").expect("outside");
    let gone = root.join("nothing-here");
    for (target, live) in [(&outside, true), (&gone, false), (&one, true)] {
        let before_one = inode(&one);
        let before_target = live.then(|| inode(target));
        std::os::unix::fs::symlink(target, tmp_two(root)).expect("a planted symlink");
        let why = withdraw_key0(root).expect_err("refused");
        assert!(
            why.contains("staging_not_regular (symlink)"),
            "{}: {why}",
            target.display()
        );
        assert!(why.contains("a symbolic link"), "{why}");
        assert_eq!(inode(&one), before_one, "epoch 1 untouched");
        match before_target {
            Some(held) => assert_eq!(inode(target), held, "{} untouched", target.display()),
            None => assert!(
                std::fs::symlink_metadata(target).is_err(),
                "a dangling link's target is never created"
            ),
        }
        assert!(
            std::fs::symlink_metadata(tmp_two(root))
                .expect("still there")
                .file_type()
                .is_symlink(),
            "the planted link is left as it is"
        );
        nothing_installed(root);
        std::fs::remove_file(tmp_two(root)).expect("the planted link removed");
    }
}

/// **A DIRECTORY OR A PIPE AT THE STAGING NAME IS REFUSED BY WHAT IT IS** and
/// left as it is; a pipe is never opened, so the install does not wait on it.
#[test]
fn a_directory_or_a_pipe_at_the_staging_name_is_refused_by_name() {
    let scratch = Scratch::new("ODD");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let before = inode(&one);
    std::fs::create_dir(tmp_two(root)).expect("a directory at the staging name");
    let why = withdraw_key0(root).expect_err("refused");
    assert!(why.contains("staging_not_regular (directory)"), "{why}");
    nothing_installed(root);
    std::fs::remove_dir(tmp_two(root)).expect("removed");

    // `mkfifo` is the external program, run from a test only.
    let made = std::process::Command::new("mkfifo")
        .arg(tmp_two(root))
        .status()
        .expect("mkfifo ran");
    assert!(made.success());
    let at = root.to_path_buf();
    let why = answered_without_waiting(move || withdraw_key0(&at)).expect_err("refused");
    assert!(why.contains("staging_not_regular (pipe)"), "{why}");
    assert!(
        std::fs::symlink_metadata(tmp_two(root))
            .expect("still there")
            .file_type()
            .is_fifo(),
        "left as it is"
    );
    assert_eq!(inode(&one), before);
    nothing_installed(root);
}

/// **A DEAD WRITER'S LEFTOVER — A REGULAR FILE WITH ONE NAME — IS CLEARED
/// AND THE INSTALL LANDS**, of 0 bytes and of a torn image alike: the new
/// epoch holds the new census, not the leftover's bytes, and epoch 1 is kept.
#[test]
fn a_stale_regular_staging_file_is_cleared_and_the_install_lands() {
    for leftover in [&b""[..], &b"half an epoch, invented"[..]] {
        let scratch = Scratch::new("STALE");
        let root = scratch.root();
        let one = epoch_one(root, 8, 4);
        let before = inode(&one);
        std::fs::write(tmp_two(root), leftover).expect("a leftover");
        let done = withdraw_key0(root)
            .expect("installed")
            .expect("a compaction");
        assert_eq!((done.from_epoch, done.to_epoch), (Some(1), 2));
        let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
        assert_eq!(
            read_epoch(&two, Vendor::Gdfl, Segment::Fno)
                .expect("epoch 2 reads")
                .entries(),
            3,
            "key 0 withdrawn"
        );
        let meta = std::fs::symlink_metadata(&two).expect("epoch 2");
        assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
        assert_ne!(meta.ino(), before.1, "not epoch 1's inode");
        assert!(std::fs::symlink_metadata(tmp_two(root)).is_err(), "renamed");
        assert_eq!(inode(&one), before, "epoch 1 kept byte for byte");
        assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));
    }
}

/// **A FULL DISK DURING THE WRITE AND A KILL BEFORE THE RENAME RENAME
/// NOTHING**, and the next install clears what they left and lands. The
/// failpoint stands where the host's `write` and `rename` would fail.
#[test]
fn a_full_disk_or_a_kill_mid_install_renames_nothing_and_the_next_install_lands() {
    for (step, word) in [
        (WholeStep::Write, "could not be written"),
        (WholeStep::Rename, "could not be renamed"),
    ] {
        let scratch = Scratch::new("FAIL");
        let root = scratch.root();
        let one = epoch_one(root, 8, 4);
        let before = inode(&one);
        let census = read_epoch(&one, Vendor::Gdfl, Segment::Fno).expect("epoch 1");
        let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
        let lock = target.lock().expect("the lock");
        let mut fail = |at: WholeStep| {
            if at == step {
                // ENOSPC, the host's "No space left on device".
                Err(std::io::Error::from_raw_os_error(28))
            } else {
                Ok(())
            }
        };
        let why = install_epoch_with(
            &lock,
            root,
            Vendor::Gdfl,
            Segment::Fno,
            2,
            &census.image(),
            &mut fail,
        )
        .expect_err("stopped");
        drop(lock);
        assert!(why.contains(word), "{step:?}: {why}");
        assert!(why.contains("nothing was renamed"), "{why}");
        assert!(why.contains("os error 28"), "{why}");
        assert_eq!(inode(&one), before, "epoch 1 intact");
        nothing_installed(root);
        let left = std::fs::symlink_metadata(tmp_two(root)).expect("the staging file is left");
        assert!(
            left.is_file() && left.nlink() == 1,
            "a dead writer's leftover"
        );

        let done = withdraw_key0(root).expect("lands").expect("a compaction");
        assert_eq!(done.to_epoch, 2);
        assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));
        assert_eq!(inode(&one), before, "epoch 1 still intact");
    }
}

/// **A READ-ONLY PARTITION DIRECTORY IS NAMED, NOT HIDDEN.** The host's
/// `EACCES` comes back with the staging name; nothing is written.
#[test]
fn a_read_only_partition_directory_is_refused_with_the_host_s_words() {
    let scratch = Scratch::new("EACCES");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let before = inode(&one);
    let dir = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    let why = withdraw_key0(root);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("restored");
    let why = why.expect_err("refused");
    assert!(why.contains("io: create"), "{why}");
    assert!(why.contains("Permission denied"), "{why}");
    assert!(why.contains(&tmp_two(root).display().to_string()), "{why}");
    assert_eq!(inode(&one), before);
    nothing_installed(root);
}

/// **TWO INSTALLERS AT ONCE: THE SECOND IS REFUSED AT THE PARTITION'S LOCK**,
/// so no two writers ever share one staging name; the lock released, the
/// install lands.
#[test]
fn a_second_installer_is_refused_at_the_lock_and_writes_nothing() {
    let scratch = Scratch::new("TWO");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let before = inode(&one);
    let held = Target::partition(root, Vendor::Gdfl, Segment::Fno)
        .lock()
        .expect("the first installer's lock");
    let why = withdraw_key0(root).expect_err("refused at the lock");
    assert!(why.contains("holds the census lock"), "{why}");
    assert!(why.contains("FNO.lock"), "{why}");
    assert!(
        std::fs::symlink_metadata(tmp_two(root)).is_err(),
        "no staging file"
    );
    assert_eq!(inode(&one), before);
    nothing_installed(root);
    drop(held);
    assert!(withdraw_key0(root).expect("lands").is_some());
}

/// **A SYMBOLIC LINK AT AN EPOCH'S NAME IS REFUSED BY NAME** by the listing
/// and by the read, never answered as the current epoch; a directory too.
/// Before the fix the listing answered `Ok(Some(2))` for it and only the
/// read's ELOOP stopped the partition.
#[test]
fn current_epoch_refuses_a_symlinked_epoch_by_name() {
    let scratch = Scratch::new("EPOCHLINK");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    std::os::unix::fs::symlink(&one, &two).expect("a symlinked epoch");
    let why = current_epoch(root, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    assert!(why.contains(&two.display().to_string()), "{why}");
    assert!(why.contains("epoch_not_regular (symlink): "), "{why}");
    let why = read_epoch(&two, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    assert!(why.contains("epoch_not_regular (symlink): "), "{why}");
    assert!(!why.contains("levels of symbolic links"), "named: {why}");
    // A write to the partition is refused by the same sentence, nothing done.
    let before = inode(&one);
    let why = withdraw_key0(root).expect_err("refused");
    assert!(why.contains("epoch_not_regular (symlink): "), "{why}");
    assert_eq!(inode(&one), before);
    std::fs::remove_file(&two).expect("the link removed");

    std::fs::create_dir(&two).expect("a directory at an epoch's name");
    let why = current_epoch(root, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    assert!(why.contains("epoch_not_regular (directory): "), "{why}");
    std::fs::remove_dir(&two).expect("removed");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(1)));
}

/// **AN APPEND NEVER WRITES INTO AN EPOCH THAT IS A SECOND NAME OF ANOTHER**
/// (AG-05 repair 1; the second check's `probe_pre_damaged_shared_inode_epochs`).
/// The state the old staging open left behind — epoch 2 a hard link of epoch
/// 1's inode — is refused by name at the batch's open. Before the fix
/// `record` answered `Ok` and epoch 1's bytes changed under it. The open
/// itself refuses a symbolic link and a pipe by name, without waiting, and
/// opens a regular file of one name.
#[test]
fn an_append_never_writes_into_an_epoch_with_a_second_name() {
    let scratch = Scratch::new("SHARED");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    std::fs::hard_link(&one, &two).expect("epoch 2 a second name of epoch 1");
    let before = inode(&one);
    let why = record(root, Vendor::Gdfl, &[row(7, 3)]).expect_err("refused");
    assert!(why.contains(&two.display().to_string()), "{why}");
    assert!(why.contains("has 2 names (a hard link)"), "{why}");
    assert!(why.contains("nothing was written"), "{why}");
    assert_eq!(inode(&one), before, "epoch 1 kept, its inode and its bytes");
    assert_eq!(inode(&two).3, before.3, "epoch 2's name unchanged");

    // THE OPEN ITSELF, at each other thing that can stand at the name.
    let link = root.join("a-link.man");
    std::os::unix::fs::symlink(&one, &link).expect("a link");
    let why = super::open_for_batch(&link)
        .expect_err("refused")
        .to_string();
    assert!(why.contains(&link.display().to_string()), "{why}");
    assert!(why.contains("is a symbolic link"), "{why}");
    let pipe = root.join("a-pipe.man");
    let made = std::process::Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .expect("mkfifo ran");
    assert!(made.success());
    let at = pipe.clone();
    let why = answered_without_waiting(move || super::open_for_batch(&at).map(|_| ()))
        .expect_err("refused")
        .to_string();
    assert!(why.contains("is not a regular file"), "{why}");
    std::fs::remove_file(&two).expect("the second name moved aside");
    assert!(
        super::open_for_batch(&one).is_ok(),
        "one name, a regular file"
    );
    assert_eq!(inode(&one).3, before.3, "opening wrote nothing");
}

/// **EVERY CALLER OF THE EPOCH INSTALL INHERITS THE REFUSAL**: the batch's
/// compaction of a full partition, the recount's whole publish
/// ([`Mode::PerKey`], what `cli census-recount` runs), the run-level
/// compaction of an import, and the re-key's withdraw. Each meets a hard link
/// to epoch 1 at epoch 2's staging name and leaves epoch 1 as it was.
#[test]
fn compaction_recount_and_withdraw_each_refuse_a_planted_link() {
    type Caller = fn(&Path) -> Result<(), String>;
    let callers: [(&str, Caller); 4] = [
        ("compaction", |root| {
            let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
            cycle(&target, &[row(2, 1)], Mode::AllOrNothing).map(|_| ())
        }),
        ("recount", |root| {
            let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
            cycle(&target, &[row(2, 1)], Mode::PerKey).map(|_| ())
        }),
        ("import run", |root| {
            let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
            let lock = target.lock()?;
            let loaded = target.load()?;
            install_run(&lock, &target, &loaded, &[], false, true)
        }),
        ("withdraw", |root| withdraw_key0(root).map(|_| ())),
    ];
    for (caller, run) in callers {
        let scratch = Scratch::new("CALLERS");
        let root = scratch.root();
        // Full at a capacity of two distinct keys: a new row compacts.
        let one = epoch_one(root, 2, 2);
        std::fs::hard_link(&one, tmp_two(root)).expect("a planted hard link");
        let before = inode(&one);
        let why = run(root).expect_err(caller);
        assert!(why.contains("staging_linked"), "{caller}: {why}");
        assert_eq!(inode(&one), before, "{caller}: epoch 1 as it was");
        nothing_installed(root);
        // The link moved aside, the same call lands.
        std::fs::remove_file(tmp_two(root)).expect("the planted link removed");
        run(root).unwrap_or_else(|why| panic!("{caller}: {why}"));
        assert_eq!(
            current_epoch(root, Vendor::Gdfl, Segment::Fno),
            Ok(Some(2)),
            "{caller}"
        );
        assert_eq!(inode(&one).3, before.3, "{caller}: epoch 1's bytes kept");
    }
}

/// **THE SEAL'S STUB AND ITS RECEIPT ARE BORN THE SAME WAY.** A hard link at
/// `<vendor>.man.seal.tmp` refuses the seal before the census's name is
/// touched; one at `SEALED.tmp` refuses the receipt after the stub, and the
/// linked file keeps its bytes. Moved aside, a rerun finishes the seal.
#[test]
fn the_seal_s_stub_and_receipt_refuse_a_planted_link() {
    let scratch = Scratch::new("SEAL");
    let root = scratch.root();
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    let done = migrate(root, Vendor::Gdfl).expect("it migrates");
    assert!(done.seal.as_ref().is_some_and(Result::is_ok));
    // Back to the state of a store migrated before the seal existed.
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = superseded_path(root, Vendor::Gdfl);
    let receipt = vendor_dir(root, Vendor::Gdfl).join(SEAL_RECEIPT);
    std::fs::remove_file(&legacy).expect("the stub removed");
    std::fs::rename(&kept, &legacy).expect("the census back at its name");
    std::fs::remove_file(&receipt).expect("no receipt");
    let census = std::fs::read(&legacy).expect("the census");

    let witness = root.join("witness.txt");
    std::fs::write(&witness, b"an invented file the seal must not write").expect("witness");
    let mut stub_tmp = legacy.clone().into_os_string();
    stub_tmp.push(".seal.tmp");
    let stub_tmp = PathBuf::from(stub_tmp);
    std::fs::hard_link(&witness, &stub_tmp).expect("planted at the stub's staging name");
    // Measured with the planted name: two links, one inode, the same bytes.
    let witness_before = inode(&witness);
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("staging_linked"), "{why}");
    assert!(why.contains("still holds the census"), "{why}");
    assert_eq!(
        inode(&witness),
        witness_before,
        "the linked file is unchanged"
    );
    assert_eq!(
        std::fs::read(&legacy).expect("legacy"),
        census,
        "the census name untouched"
    );
    std::fs::remove_file(&stub_tmp).expect("moved aside");

    let mut receipt_tmp = receipt.clone().into_os_string();
    receipt_tmp.push(".tmp");
    let receipt_tmp = PathBuf::from(receipt_tmp);
    std::fs::hard_link(&witness, &receipt_tmp).expect("planted at the receipt's staging name");
    let witness_before = inode(&witness);
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("staging_linked"), "{why}");
    assert_eq!(
        inode(&witness),
        witness_before,
        "the linked file is unchanged"
    );
    assert!(std::fs::symlink_metadata(&receipt).is_err(), "no receipt");
    std::fs::remove_file(&receipt_tmp).expect("moved aside");

    let seal = seal_legacy(root, Vendor::Gdfl).expect("the rerun finishes it");
    assert!(seal.wrote);
    assert_eq!(std::fs::read(&seal.superseded).expect("kept"), census);
    assert!(std::fs::symlink_metadata(&receipt).is_ok_and(|m| m.is_file()));
}

/// **`manifest/REQUIRES` IS NEVER APPENDED THROUGH A LINK, AND NEVER WAITED
/// ON** (AG-05 repair 1, the second agent's follow-up). The append opened the
/// name with `create` and `append`, following a link: a symbolic link there
/// grew the file it named, outside the store, a dangling one created its
/// target, and a hard link grew another name's bytes; the read before it
/// (`read_to_string`) would wait forever on a pipe. Each is refused now,
/// naming the path, with nothing written anywhere; a name with nothing at it
/// still gets its one requirement, once.
#[test]
fn the_requires_append_never_writes_through_a_link() {
    let scratch = Scratch::new("REQ");
    let root = scratch.root();
    let at = requires_path(root);
    let named = at.display().to_string();
    let witness = root.join("witness.txt");
    let said = b"an invented file no requirement may grow".to_vec();
    std::fs::write(&witness, &said).expect("witness");

    // A SYMBOLIC LINK AT THE NAME, to a file outside the manifest.
    std::os::unix::fs::symlink(&witness, &at).expect("a link");
    let why = require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4).expect_err("refused");
    assert!(why.contains(&named), "{why}");
    assert!(why.contains("is a symbolic link"), "{why}");
    assert!(!why.contains("levels of symbolic links"), "named: {why}");
    assert_eq!(std::fs::read(&witness).expect("witness"), said, "unchanged");
    std::fs::remove_file(&at).expect("moved aside");

    // A DANGLING ONE: nothing is created at the name it points to.
    let nowhere = root.join("nowhere.txt");
    std::os::unix::fs::symlink(&nowhere, &at).expect("a dangling link");
    let why = require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4).expect_err("refused");
    assert!(why.contains("is a symbolic link"), "{why}");
    assert!(std::fs::symlink_metadata(&nowhere).is_err(), "nothing made");
    std::fs::remove_file(&at).expect("moved aside");

    // A HARD LINK AT THE NAME: one inode with the outside file.
    std::fs::hard_link(&witness, &at).expect("a hard link");
    let why = require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4).expect_err("refused");
    assert!(why.contains(&named), "{why}");
    assert!(why.contains("has 2 names"), "{why}");
    assert_eq!(std::fs::read(&witness).expect("witness"), said, "unchanged");
    std::fs::remove_file(&at).expect("moved aside");

    // A PIPE AT THE NAME: refused at once, never waited on, left as it is.
    let made = std::process::Command::new("mkfifo")
        .arg(&at)
        .status()
        .expect("mkfifo ran");
    assert!(made.success());
    let held = root.to_path_buf();
    let why =
        answered_without_waiting(move || require_feature(&held, Vendor::Gdfl, FEATURE_CENSUS_V4))
            .expect_err("refused");
    assert!(why.contains("is not a regular file"), "{why}");
    assert!(
        std::fs::symlink_metadata(&at)
            .expect("still there")
            .file_type()
            .is_fifo()
    );
    std::fs::remove_file(&at).expect("moved aside");

    // NOTHING AT THE NAME: the requirement lands once, in a file of one name.
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(true)
    );
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(false),
        "once"
    );
    let text = std::fs::read_to_string(&at).expect("written");
    assert!(text.starts_with(REQUIRES_FORMAT), "{text}");
    assert_eq!(text.lines().count(), 2, "{text}");
    let meta = std::fs::symlink_metadata(&at).expect("written");
    assert!(meta.is_file() && meta.nlink() == 1);
    assert_eq!(std::fs::read(&witness).expect("witness"), said);
}

/// **AN IDEMPOTENT REQUIREMENT OPENS NOTHING FOR WRITING** (AG-05 repair 2,
/// a follow-up of verify-AG-05-r2): `require_feature` opened `REQUIRES` for
/// append before it looked whether the line was in force, so a `REQUIRES`
/// made read-only (`chmod 444`) failed a re-seal that had nothing to write
/// with the host's `EACCES`. Now the line in force answers `Ok(false)`
/// through a read-only open, and a line still to write on a read-only file
/// is refused with the path and the host's words, the file unchanged.
#[test]
fn an_idempotent_requirement_opens_nothing_for_writing() {
    let scratch = Scratch::new("REQRO");
    let root = scratch.root();
    let at = requires_path(root);
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(true)
    );
    let before = std::fs::read(&at).expect("written");
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o444)).expect("read-only");
    let again = require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4);
    let other = require_feature(root, Vendor::Zerodha, FEATURE_CENSUS_V4);
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o644)).expect("writable");
    assert_eq!(
        again,
        Ok(false),
        "in force: nothing to write, nothing opened for it"
    );
    let why = other.expect_err("a line still to write on a read-only file");
    assert!(why.contains(&at.display().to_string()), "{why}");
    assert!(why.contains("ermission denied"), "the host's words: {why}");
    assert_eq!(std::fs::read(&at).expect("unchanged"), before);
}

/// p50, p99 and max of `micros`, sorted in place.
fn percentiles(micros: &mut [u128]) -> (u128, u128, u128) {
    micros.sort_unstable();
    let n = micros.len();
    (micros[n / 2], micros[n * 99 / 100], micros[n - 1])
}

/// The install as it was before AG-05: create and truncate, following a link
/// at the name. Kept here only as the baseline the bench measures against.
fn install_as_before(dir: &Path, tmp: &Path, path: &Path, image: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::create_dir_all(dir)?;
    let mut file = std::fs::File::create(tmp)?;
    file.write_all(image)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(tmp, path)?;
    std::fs::File::open(dir)?.sync_all()
}

/// **THE STAGING RULE ADDS A CONSTANT NUMBER OF CALLS TO AN INSTALL, AND IT
/// IS MEASURED** (AG-05's O(1) proof): n = 200 epoch installs of one invented
/// partition image through the old `File::create` sequence, through
/// `install_epoch` on a free staging name (one `open`, as before), and through
/// `install_epoch` over a dead writer's leftover at every name (four calls:
/// `open`, `lstat`, `unlink`, `open`). Every install lands as the next epoch,
/// and the install's count of host calls before the image is written is
/// ASSERTED: exactly 2 on every free name and exactly 5 over every leftover
/// (R3-19 added the one look at the segment directory to the staging door's
/// 1 and 4), the same at the 1st install and the 200th. NO WALL-CLOCK BOUND
/// IS ASSERTED (AG-05 repair 1):
/// on this shared host one failed at p99 617 ms while the unchanged
/// `File::create` series beside it measured p99 309 ms, so a bound measured
/// the load, not the code. p50, p99 and max are printed for docs/06-limits.md;
/// two `fsync`s dominate all three series. Proof of the constant count: this
/// test, `pull::ingest::census_store::link_tests::an_epoch_install_through_the_door_costs_a_constant_number_of_calls_measured`
/// (docs/06-limits.md L-AG05-04, L-R319-02).
#[test]
fn an_epoch_install_through_the_door_costs_a_constant_number_of_calls_measured() {
    const N: u32 = 200;
    let scratch = Scratch::new("BENCH");
    let root = scratch.root();
    let one = epoch_one(root, 8, 4);
    let one_before = inode(&one);
    let image = read_epoch(&one, Vendor::Gdfl, Segment::Fno)
        .expect("epoch 1")
        .image();
    let dir = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    let staging = |epoch: u32| dir.join(format!("{epoch:06}.man.tmp"));

    // BEFORE: the old sequence, into names no census reads.
    let before_dir = root.join("before");
    let mut before = Vec::new();
    for n in 0..N {
        let tmp = before_dir.join(format!("{n:06}.man.tmp"));
        let path = before_dir.join(format!("{n:06}.man"));
        let start = std::time::Instant::now();
        install_as_before(&before_dir, &tmp, &path, &image).expect("the old install");
        before.push(start.elapsed().as_micros());
    }

    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let lock = target.lock().expect("the lock");
    let mut ok = |_: WholeStep| Ok(());
    let mut free = Vec::new();
    for epoch in 2..2 + N {
        let start = std::time::Instant::now();
        let (_, calls) = install_epoch_with(
            &lock,
            root,
            Vendor::Gdfl,
            Segment::Fno,
            epoch,
            &image,
            &mut ok,
        )
        .expect("installed");
        free.push(start.elapsed().as_micros());
        assert_eq!(
            calls, 2,
            "epoch {epoch}: the segment directory's look and one open on a free name (R3-19)"
        );
    }
    let mut leftover = Vec::new();
    for epoch in 2 + N..2 + 2 * N {
        std::fs::write(staging(epoch), b"a dead writer's invented leftover").expect("planted");
        let start = std::time::Instant::now();
        let (_, calls) = install_epoch_with(
            &lock,
            root,
            Vendor::Gdfl,
            Segment::Fno,
            epoch,
            &image,
            &mut ok,
        )
        .expect("installed over the leftover");
        leftover.push(start.elapsed().as_micros());
        assert_eq!(
            calls, 5,
            "epoch {epoch}: the segment directory's look, then open, lstat, unlink, open over a leftover (R3-19)"
        );
    }
    drop(lock);
    assert_eq!(
        current_epoch(root, Vendor::Gdfl, Segment::Fno),
        Ok(Some(1 + 2 * N)),
        "every install landed as the next epoch"
    );
    assert_eq!(
        std::fs::read(epoch_path(root, Vendor::Gdfl, Segment::Fno, 1 + 2 * N)).expect("last"),
        image,
        "the last epoch holds the image, not the leftover"
    );
    assert_eq!(
        inode(&one),
        one_before,
        "epoch 1 kept, its inode and its bytes"
    );
    let mut said = Vec::new();
    for (series, micros) in [
        ("before (File::create)", &mut before),
        ("after, free name", &mut free),
        ("after, over a leftover", &mut leftover),
    ] {
        let (p50, p99, max) = percentiles(micros);
        said.push(format!(
            "{series}: p50 {p50} us, p99 {p99} us, max {max} us"
        ));
    }
    eprintln!("AG-05 epoch install, n = {N} each: {}", said.join("; "));
}
