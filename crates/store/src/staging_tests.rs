//! Unit tests of `store::staging`: every row of the module's table, each on
//! a directory the test owns. Every byte written is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::Cell;
use std::fs;
use std::io::{self, ErrorKind, Read as _, Write as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Seen, StagingError, create};
use crate::safe_open::Found;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory of this test's own.
///
/// Cleared first: the name carries this process's id, and once ids wrap a
/// later process meets an earlier one's leftovers under it, which turned
/// `a_free_name_is_created_empty_and_nothing_is_cleared` into a leftover's
/// clear and the special-file test into `EEXIST` (cloud session
/// 2026-10-16, after about 10,000 test processes).
fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-staging-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _absent_is_fine = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stand-in for the look: what a race between the create and the look
/// would have the host answer.
type Stand = fn(&Path) -> io::Result<Seen>;

thread_local! {
    /// Opens to let pass on this thread before one is refused, and the kind
    /// it is refused with.
    static OPEN_REFUSAL: Cell<Option<(u8, ErrorKind)>> = const { Cell::new(None) };
    /// The stand-in that answers the next look on this thread.
    static LOOK: Cell<Option<Stand>> = const { Cell::new(None) };
}

/// Whether the armed open refusal fires now, and with which kind; it fires
/// once.
pub(super) fn open_refusal_fires() -> Option<ErrorKind> {
    OPEN_REFUSAL.with(|armed| match armed.get() {
        Some((0, kind)) => {
            armed.set(None);
            Some(kind)
        }
        Some((left, kind)) => {
            armed.set(Some((left - 1, kind)));
            None
        }
        None => None,
    })
}

/// The armed stand-in for the next look, taken: it answers once.
pub(super) fn look_stand_in() -> Option<Stand> {
    LOOK.with(Cell::take)
}

#[test]
fn a_free_name_is_created_empty_and_nothing_is_cleared() {
    let dir = dir("free");
    let path = dir.join("a.tmp");
    let mut staged = create(&path).unwrap();
    assert_eq!(staged.cleared, None);
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    staged.file.write_all(b"synthetic").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"synthetic");
    let _ = fs::remove_dir_all(&dir);
}

/// The leftover is held open across the create, so its inode cannot be
/// freed and handed to the new file: on ext4 an unlinked file's inode
/// number is reused at once, and comparing numbers alone failed there
/// (cloud session 2026-10-16, 40 of 40 runs). Held open, the old file must
/// still hold every byte it held: the create removed a NAME and cut nothing.
#[test]
fn a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported() {
    let dir = dir("leftover");
    let path = dir.join("a.tmp");
    fs::write(&path, b"half a file").unwrap();
    let mut held = fs::File::open(&path).unwrap();
    let before = held.metadata().unwrap().ino();
    let staged = create(&path).unwrap();
    assert_eq!(staged.cleared, Some(11), "the leftover's bytes are named");
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), 0, "the new file is empty");
    assert_ne!(after.ino(), before, "it is a new file, not the old one cut");
    assert_eq!(
        held.metadata().unwrap().nlink(),
        0,
        "the old file lost its name"
    );
    let mut kept = Vec::new();
    held.read_to_end(&mut kept).unwrap();
    assert_eq!(kept, b"half a file", "and kept every byte");
    let _ = fs::remove_dir_all(&dir);
}

/// **A LOOK THE HOST REFUSES AFTER THE CREATE SAID "EXISTS" IS THE LOOK'S
/// REFUSAL, AND NOTHING MOVES.**
///
/// Kills `replace match guard why.kind() == ErrorKind::NotFound with true`
/// in the look's `Err` arm, which would take any refusal of the `lstat` for
/// "it went": the create would go on to its second open, meet the leftover
/// still there and blame another writer (`staging_contended`). Only a race
/// (the directory's search permission revoked between the two calls) does
/// this on a real host, so the look's stand-in answers `EACCES`.
#[test]
fn a_look_the_host_refuses_is_a_stat_refusal_and_nothing_moves() {
    let dir = dir("look-refused");
    let path = dir.join("a.tmp");
    fs::write(&path, b"half a file").unwrap();
    LOOK.with(|armed| armed.set(Some(|_| Err(io::Error::from(ErrorKind::PermissionDenied)))));
    let refusal = create(&path).unwrap_err();
    assert!(look_stand_in().is_none(), "the look was asked once");
    assert!(
        matches!(&refusal, StagingError::Io { action: "stat", source, path: p }
            if source.kind() == ErrorKind::PermissionDenied && *p == path),
        "{refusal}"
    );
    assert_eq!(fs::read(&path).unwrap(), b"half a file", "nothing moved");
    let _ = fs::remove_dir_all(&dir);
}

/// **A NAME THAT GOES BETWEEN THE CREATE AND THE LOOK, OR WHOSE FILE THE
/// LOOK CATCHES MID-UNLINK, IS FREE, AND THE CREATE GOES ON.**
///
/// Kills `replace match guard why.kind() == ErrorKind::NotFound with false`
/// and `replace == with !=` in the look's `Err` arm (a vanished name would
/// be refused as a `stat` failure though nothing stands in the way), the
/// race `a_leftover_that_vanishes_before_the_look_leaves_the_create_free`
/// meets only by chance, here every time: the stand-ins remove the file
/// themselves, as the other writer would. Pins the second answer such a race
/// gives (cloud session 2026-10-16 saw it as "one of 0 names of a file"): a
/// regular file with no name left has none to protect, so it is not
/// `staging_linked`. Either way the create makes three host calls (the open,
/// the look, the open) and clears nothing.
#[test]
fn a_name_gone_by_the_look_or_going_during_it_is_free() {
    let dir = dir("look-gone");
    let path = dir.join("a.tmp");
    let gone: Stand = |at| {
        fs::remove_file(at).unwrap();
        Err(io::Error::from(ErrorKind::NotFound))
    };
    let going: Stand = |at| {
        fs::remove_file(at).unwrap();
        Ok(Seen {
            found: None,
            links: 0,
            len: 11,
        })
    };
    for (stand, what) in [(gone, "gone"), (going, "going")] {
        fs::write(&path, b"half a file").unwrap();
        LOOK.with(|armed| armed.set(Some(stand)));
        let mut staged = create(&path).unwrap_or_else(|refusal| panic!("{what}: {refusal}"));
        assert!(look_stand_in().is_none(), "{what}: the look was asked once");
        assert_eq!(
            (staged.cleared, staged.calls),
            (None, 3),
            "{what}: nothing was cleared"
        );
        staged.file.write_all(b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new", "{what}: the new file");
        fs::remove_file(&path).unwrap();
    }
    // A FILE CAUGHT GOING WHOSE NAME IS TAKEN AGAIN: another writer.
    fs::write(&path, b"another writer's").unwrap();
    LOOK.with(|armed| {
        armed.set(Some(|_| {
            Ok(Seen {
                found: None,
                links: 0,
                len: 0,
            })
        }));
    });
    let refusal = create(&path).unwrap_err();
    assert!(
        matches!(&refusal, StagingError::Contended { path: p } if *p == path),
        "{refusal}"
    );
    assert_eq!(fs::read(&path).unwrap(), b"another writer's", "left whole");
    let _ = fs::remove_dir_all(&dir);
}

/// **A SECOND CREATE THE HOST REFUSES OTHER THAN "EXISTS" IS A CREATE
/// REFUSAL, NOT ANOTHER WRITER.**
///
/// Kills `replace match guard why.kind() == ErrorKind::AlreadyExists with
/// true` in the second open, which would answer every refusal of it with
/// `staging_contended` ("another writer is using the name") when the host
/// refused the create itself. On a real host only a change to the
/// directory between the clear and the create does this, so the open's
/// seam lets the first open through and refuses the second with `EACCES`.
/// The leftover's NAME is gone by then: the clear ran, and said so in no
/// byte of the file.
#[test]
fn a_second_create_the_host_refuses_is_a_create_refusal_not_another_writer() {
    let dir = dir("second-open");
    let path = dir.join("a.tmp");
    fs::write(&path, b"half a file").unwrap();
    let mut held = fs::File::open(&path).unwrap();
    OPEN_REFUSAL.with(|armed| armed.set(Some((1, ErrorKind::PermissionDenied))));
    let refusal = create(&path).unwrap_err();
    assert!(
        open_refusal_fires().is_none(),
        "the second open was refused"
    );
    assert!(
        matches!(&refusal, StagingError::Io { action: "create", source, path: p }
            if source.kind() == ErrorKind::PermissionDenied && *p == path),
        "{refusal}"
    );
    assert!(!path.exists(), "the leftover's name was cleared");
    let mut kept = Vec::new();
    held.read_to_end(&mut kept).unwrap();
    assert_eq!(kept, b"half a file", "and not one of its bytes");
    let _ = fs::remove_dir_all(&dir);
}

/// Every temporary name the tree spells today, as its writer builds it, and
/// the one the ingest design fixes for a stop file (RA-287), each beside the
/// writer or the source that names it. The helper's answer must not depend
/// on a name's shape: dots, a process number, a run, a stem in capitals.
/// The writers here that still open their temporaries their own way are the
/// legacy list of `.github/gates_tmp_open.rs`, which fails any NEW one; a
/// package that names a new temporary adds its row here when it does.
const TEMPORARY_NAMES: [(&str, &str); 10] = [
    ("store::verdict, a year index", "2031.vdi.4242-7.tmp"),
    (
        "store::verdict, a day image",
        "2031-03-04.g0000000001.vdt.4242-7.tmp",
    ),
    ("pull::census_store, an epoch", "000003.man.tmp"),
    ("pull::census_store, a seal stub", "gdfl.man.seal.tmp"),
    ("pull::census_store, the sealed text", "SEALED.tmp"),
    ("pull::alerts, a queue's birth", "alerts.4242.tmp"),
    (
        "pull::gdfl_tickstore, a day file",
        "GFDLCM_INDICES_TICK_01042024.bts.tmp",
    ),
    ("cli::live, a published result", "6b6b6b6b.4242.tmp"),
    ("cli::search_checkpoint, a completion", "complete.tmp"),
    (
        "design (ingest-automation.md:1376), a stop file",
        "gdfl.options.stop.tmp",
    ),
];

/// **RA-290, RA-287: A HARD LINK AT A STAGING NAME IS REFUSED, AND THE FILE
/// BEHIND IT IS UNTOUCHED** — at every temporary name in one table. A second
/// name of another file planted at the temporary is never opened: the create
/// is refused `staging_linked` naming the name and the count of names, not
/// one byte of the file behind it changes, and the planted name is left for
/// the operator to see. The same names with nothing there are created, so
/// the refusal is the link's and not the name's.
#[test]
fn a_hard_link_at_a_staging_name_is_refused_and_the_file_behind_it_is_untouched() {
    let dir = dir("linked");
    let body = b"bytes that must survive".to_vec();
    for (writer, name) in TEMPORARY_NAMES {
        let victim = dir.join(format!("victim-of-{name}"));
        fs::write(&victim, &body).unwrap();
        let path = dir.join(name);
        fs::hard_link(&victim, &path).unwrap();

        let refusal = create(&path).unwrap_err();
        assert_eq!(refusal.word(), "staging_linked", "{writer}: {refusal}");
        assert!(
            matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
            "{writer}: {refusal}"
        );
        assert_eq!(refusal.path(), path, "{writer}");
        assert!(refusal.to_string().contains("one of 2 names"), "{refusal}");
        assert_eq!(
            fs::read(&victim).unwrap(),
            body,
            "{writer}: not one byte changed"
        );
        assert_eq!(
            fs::metadata(&victim).unwrap().nlink(),
            2,
            "{writer}: the planted name is left for the operator to see"
        );

        // THE SAME NAME, FREE: created empty, so the refusal above was the
        // link's.
        fs::remove_file(&path).unwrap();
        let made = create(&path).unwrap();
        assert_eq!((made.cleared, made.calls), (None, 1), "{writer}");
        assert_eq!(fs::metadata(&path).unwrap().nlink(), 1, "{writer}");
        assert_eq!(fs::read(&victim).unwrap(), body, "{writer}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_symbolic_link_or_a_directory_at_a_staging_name_is_refused_and_left() {
    let dir = dir("special");
    let victim = dir.join("victim.bin");
    fs::write(&victim, b"kept").unwrap();
    let link = dir.join("link.tmp");
    std::os::unix::fs::symlink(&victim, &link).unwrap();
    let refusal = create(&link).unwrap_err();
    assert_eq!(refusal.word(), "staging_not_regular", "{refusal}");
    assert!(
        matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
        "{refusal}"
    );
    assert!(refusal.to_string().contains("a symbolic link"), "{refusal}");
    assert_eq!(fs::read(&victim).unwrap(), b"kept");
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());

    let dangling = dir.join("dangling.tmp");
    std::os::unix::fs::symlink(dir.join("nowhere"), &dangling).unwrap();
    assert_eq!(
        create(&dangling).unwrap_err().word(),
        "staging_not_regular",
        "a link to nothing is not created through"
    );
    assert!(!dir.join("nowhere").exists());

    let folder = dir.join("folder.tmp");
    fs::create_dir(&folder).unwrap();
    assert_eq!(create(&folder).unwrap_err().word(), "staging_not_regular");
    assert!(folder.is_dir());

    // A PIPE AND A SOCKET are named by what they are, and left.
    let pipe = dir.join("pipe.tmp");
    let made = std::process::Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());
    let socket = dir.join("socket.tmp");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    for (at, found) in [(&pipe, Found::Pipe), (&socket, Found::Socket)] {
        let refusal = create(at).unwrap_err();
        assert!(
            matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
            "{refusal}"
        );
        assert!(refusal.to_string().contains(found.sentence()), "{refusal}");
        assert!(fs::symlink_metadata(at).is_ok(), "left as it is");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_directory_is_the_hosts_refusal_by_name() {
    let dir = dir("nodir");
    let path = dir.join("absent").join("a.tmp");
    let refusal = create(&path).unwrap_err();
    assert_eq!(refusal.word(), "io");
    assert!(
        matches!(
            &refusal,
            StagingError::Io {
                action: "create",
                ..
            }
        ),
        "{refusal}"
    );
    assert!(std::error::Error::source(&refusal).is_some());
    let _ = fs::remove_dir_all(&dir);
}

/// The cost the module states: one host call on a free name and four over a
/// leftover, whatever the leftover holds (nothing, or 4 MiB) and however
/// many names share the directory (none, or 2,000). No call is made per
/// byte or per name.
#[test]
fn a_create_makes_one_host_call_on_a_free_name_and_four_over_a_leftover() {
    let quiet = dir("calls-quiet");
    let crowded = dir("calls-crowded");
    for i in 0..2_000 {
        fs::write(crowded.join(format!("zz-other-{i:04}")), b"").unwrap();
    }
    for within in [&quiet, &crowded] {
        let free = within.join("zz-free.tmp");
        let made = create(&free).unwrap();
        assert_eq!((made.calls, made.cleared), (1, None));
        for held in [0usize, 4 << 20] {
            let name = within.join(format!("zz-left-{held}.tmp"));
            fs::write(&name, vec![0x5A; held]).unwrap();
            let made = create(&name).unwrap();
            assert_eq!(
                (made.calls, made.cleared),
                (4, Some(u64::try_from(held).unwrap())),
                "a leftover of {held} bytes in {}",
                within.display()
            );
            assert_eq!(fs::metadata(&name).unwrap().len(), 0);
        }
    }
    let _ = fs::remove_dir_all(&quiet);
    let _ = fs::remove_dir_all(&crowded);
}
