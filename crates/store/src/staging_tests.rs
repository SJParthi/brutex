//! Unit tests of `store::staging`: every row of the module's table, each on
//! a directory the test owns. Every byte written is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{StagingError, create};
use crate::safe_open::Found;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory of this test's own.
fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-staging-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
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
}

#[test]
fn a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported() {
    let dir = dir("leftover");
    let path = dir.join("a.tmp");
    fs::write(&path, b"half a file").unwrap();
    let before = fs::metadata(&path).unwrap().ino();
    let staged = create(&path).unwrap();
    assert_eq!(staged.cleared, Some(11), "the leftover's bytes are named");
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), 0, "the new file is empty");
    assert_ne!(after.ino(), before, "it is a new file, not the old one cut");
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
        "gdfl.options.stop.4242-7.tmp",
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
    // Bound through the store's one short-address helper: under a long
    // TMPDIR the direct bind failed before the refusal was reached (GF-1).
    let _listener = crate::safe_open::tests::socket_at(&socket, "staging");
    for (at, found) in [(&pipe, Found::Pipe), (&socket, Found::Socket)] {
        let refusal = create(at).unwrap_err();
        assert!(
            matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
            "{refusal}"
        );
        assert!(refusal.to_string().contains(found.sentence()), "{refusal}");
        assert!(fs::symlink_metadata(at).is_ok(), "left as it is");
    }
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
