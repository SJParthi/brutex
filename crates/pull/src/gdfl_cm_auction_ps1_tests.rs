#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-PULL-PS1 (D-7335): the arms of the auction sidecar no test
//! reached at tip 2e7a0b6d3b56 (Route B), each driven by the input that
//! reaches it: a field put past the end of its buffer, a descriptor the host
//! will not `flock` (Linux `O_PATH`), a relative store root too long to name,
//! and a sidecar path that is a link to `/dev/null`, which takes an append
//! and refuses both its sync and its undo; and, handed to `append_with`, a
//! lock release and a torn-tail cut the host refuses. The share
//! (`ZQAUCTION`), its prices and quantities are invented; no vendor row is
//! quoted.

use brutex_core::instrument::Exchange;

use super::*;

fn day(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).unwrap()
}

fn key() -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, "ZQAUCTION").unwrap()
}

/// One invented print at 15:29:03, at a price no share of that name had.
fn one_print() -> [Print; 1] {
    [Print {
        sod: 15 * 3_600 + 29 * 60 + 3,
        price: 777_700,
        quantity: 77,
        rows: 1,
        line: 2,
    }]
}

fn sidecar_why(result: Result<Appended, CmRefusal>) -> String {
    match result {
        Err(CmRefusal::AuctionSidecar { why }) => why,
        other => panic!("not a sidecar refusal: {other:?}"),
    }
}

/// A field put past the end of its buffer is not written and no byte of the
/// buffer moves; one that fits is written whole, where it was put.
#[test]
fn a_field_past_the_end_of_its_buffer_writes_nothing() {
    let mut out = [7_u8; 4];
    put(&mut out, 3, &[1, 2]);
    assert_eq!(out, [7; 4], "one byte past the end: nothing written");
    put(&mut out, 2, &[1, 2]);
    assert_eq!(out, [7, 7, 1, 2]);
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

/// A sidecar descriptor the host will not `flock` is refused by `hold` at
/// once, on either side, in the host's own words: never waited on as a busy
/// lock, never taken as held. Linux only (`O_PATH`).
#[cfg(target_os = "linux")]
#[test]
fn a_descriptor_the_host_will_not_flock_is_refused_in_its_words() {
    let root = crate::gdfl_fixtures::scratch("ps1-auc-unflockable");
    let lock_at = root.join("ZQAUCTION.auc");
    std::fs::write(&lock_at, b"").unwrap();
    // The premise, in the host's own words for each side.
    let Err(std::fs::TryLockError::Error(shared)) = unlockable(&lock_at).try_lock_shared() else {
        panic!("the host took a shared flock on an O_PATH descriptor");
    };
    let Err(std::fs::TryLockError::Error(exclusive)) = unlockable(&lock_at).try_lock() else {
        panic!("the host took an exclusive flock on an O_PATH descriptor");
    };
    for (kind, host) in [(Hold::Shared, shared), (Hold::Exclusive, exclusive)] {
        assert_eq!(
            hold(&unlockable(&lock_at), &lock_at, kind, LOCK_WAIT).err(),
            Some(format!("cannot be locked: {host}")),
            "{kind:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// An append under a relative store root whose first folder is longer than
/// any path the host takes walks the folders it would make up to the working
/// folder (the empty path, where the walk stops), and is refused when the
/// host refuses to make the first of them, naming the sidecar's folder in
/// the host's words (ENAMETOOLONG). Nothing is made, anywhere.
#[test]
fn an_append_under_a_relative_root_too_long_to_name_is_refused_naming_its_folder() {
    let long = "Z".repeat(5_000);
    let root = Path::new(&long);
    let on = day(2026, 9, 23);
    let folder = path(root, &key(), on)
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    // The premise: the host refuses the root's own name.
    let host = std::fs::create_dir(root).unwrap_err();
    assert_eq!(
        sidecar_why(append(root, &key(), on, &one_print())),
        format!("{}: {host}", folder.display())
    );
}

/// An append whose write cannot be made durable, and whose undo the host
/// refuses too, says both and that the file may end torn: the sidecar's path
/// is a link to `/dev/null`, which takes the bytes and refuses `fdatasync`
/// and `ftruncate` alike (EINVAL: a character device neither syncs nor
/// truncates), as root too. The link is left as it was. Linux only.
#[cfg(target_os = "linux")]
#[test]
fn a_write_that_can_be_neither_synced_nor_undone_says_both() {
    let root = crate::gdfl_fixtures::scratch("ps1-auc-dev-null");
    let on = day(2026, 9, 23);
    let link = path(&root, &key(), on).unwrap();
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("/dev/null", &link).unwrap();
    // The premises, in the host's own words.
    let null = OpenOptions::new()
        .read(true)
        .append(true)
        .open("/dev/null")
        .unwrap();
    let unsynced = null.sync_data().unwrap_err();
    let uncut = null.set_len(0).unwrap_err();
    assert_eq!(
        sidecar_why(append(&root, &key(), on, &one_print())),
        format!(
            "{}: {unsynced}; undoing the append to 0 bytes failed too ({uncut}): the file \
             may end in a torn tail every reader refuses",
            link.display()
        )
    );
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link is left as it was"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// An append whose lock the host will not release is refused, naming the
/// sidecar and the host's words, after the day's records were written and
/// synced under it: they are held, and the next append of the day writes
/// nothing. The release `append` hands `append_with`, refused by a test.
#[test]
fn an_append_whose_lock_is_not_released_is_refused_by_name() {
    let root = crate::gdfl_fixtures::scratch("ps1-auc-unreleased");
    let on = day(2026, 9, 23);
    let at = path(&root, &key(), on).unwrap();
    let refusal = || store::flock::Unreleased {
        path: at.clone(),
        why: io::Error::other("ZQ invented: not unlocked"),
    };
    let host = Host {
        release: &|_lock| Err(refusal()),
        cut: &|_, _| unreachable!("a fresh sidecar has no torn tail to cut"),
    };
    assert_eq!(
        sidecar_why(append_with(&root, &key(), on, &one_print(), &host)),
        format!("{}: the lock was not released: {}", at.display(), refusal())
    );
    assert_eq!(read(&at).unwrap().unwrap().records.len(), 1, "written");
    assert_eq!(append(&root, &key(), on, &one_print()).unwrap().written, 0);
    std::fs::remove_dir_all(root).unwrap();
}

/// An append that finds a torn tail the host will not cut back is refused,
/// naming the sidecar, the bytes it would cut and the host's words, and
/// writes nothing: the file is left as it was, torn tail and all, for the
/// next append. The cut `append` hands `append_with`, refused by a test.
#[test]
fn a_torn_tail_the_host_will_not_cut_back_refuses_the_append() {
    let root = crate::gdfl_fixtures::scratch("ps1-auc-uncut");
    let (first, next) = (day(2026, 9, 22), day(2026, 9, 23));
    let at = path(&root, &key(), first).unwrap();
    append(&root, &key(), first, &one_print()).unwrap();
    let mut torn = std::fs::read(&at).unwrap();
    let record = Record::of(next, &one_print()[0]).encode(VERSION, 2);
    torn.extend_from_slice(record.get(..17).unwrap());
    std::fs::write(&at, &torn).unwrap();
    let host = Host {
        release: &|lock| lock.release(),
        cut: &|_, _| Err(io::Error::other("ZQ invented: not cut")),
    };
    assert_eq!(
        sidecar_why(append_with(&root, &key(), next, &one_print(), &host)),
        format!(
            "{}: its torn tail (17 of {} bytes) could not be cut back: ZQ invented: not cut",
            at.display(),
            torn.len()
        )
    );
    assert_eq!(
        std::fs::read(&at).unwrap(),
        torn,
        "nothing cut, nothing written"
    );
    std::fs::remove_dir_all(root).unwrap();
}
