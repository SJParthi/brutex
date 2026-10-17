//! Unit tests of `store::safe_open`: every row of the module's table, each
//! on a directory the test owns. Every byte written is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::{
    BARS_DIR, Found, ReadRefusal, StoreRootRefusal, open_read, open_read_len, open_read_write,
    refuse_linked_store_root,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory of this test's own.
fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-safe-open-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _absent_is_fine = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A real FIFO at `at`, made by `mkfifo` (test-only, as `tests/fifo.rs`).
fn mkfifo(at: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(at)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", at.display());
}

/// Runs `open` on its own thread and fails the test if it has not answered
/// within ten seconds: an open that waits on a pipe is released by opening
/// the pipe's other end, then named.
fn answers_at_once<T: Send + 'static>(pipe: &Path, open: impl FnOnce() -> T + Send + 'static) -> T {
    let (send, answer) = std::sync::mpsc::channel();
    let _thread = std::thread::spawn(move || send.send(open()));
    let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
        let _release = fs::OpenOptions::new().write(true).open(pipe);
        panic!("an open of {} waited for a writer", pipe.display());
    };
    answered
}

/// The first block device this host shows, if any: a disk node that an
/// unprivileged open is refused (`EACCES`), which the door must still name.
fn a_block_device() -> Option<PathBuf> {
    use std::os::unix::fs::FileTypeExt as _;
    [
        "/dev/disk0",
        "/dev/sda",
        "/dev/vda",
        "/dev/nvme0n1",
        "/dev/loop0",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|at| fs::symlink_metadata(at).is_ok_and(|m| m.file_type().is_block_device()))
}

/// What both doors answer at `path`.
fn both(path: &Path) -> [Result<u64, ReadRefusal>; 2] {
    [
        open_read_len(path).map(|(_, len)| len),
        open_read_write(path).map(|(_, len)| len),
    ]
}

#[test]
fn a_regular_file_opens_with_its_length_whatever_its_names() {
    let dir = dir("regular");
    let path = dir.join("evidence.bin");
    fs::write(&path, b"synthetic bytes").unwrap();
    assert_eq!(open_read(&path).unwrap().metadata().unwrap().len(), 15);
    for answered in both(&path) {
        assert_eq!(answered.unwrap(), 15);
    }
    // A SECOND NAME IS NOT A REFUSAL for a reader: reading through it
    // changes no byte (RA-317: "not 'exactly one link'").
    let second = dir.join("second-name.bin");
    fs::hard_link(&path, &second).unwrap();
    assert_eq!(fs::metadata(&second).unwrap().nlink(), 2);
    assert_eq!(open_read_len(&second).unwrap().1, 15);
    let _ = fs::remove_dir_all(&dir);
}

/// **A PIPE, A DEVICE OR A LINK IS REFUSED BY NAME** (R3-16, RA-317). One
/// table over every kind a name can hold that is not a regular file: each
/// is refused `read_not_regular` naming what it found and the name, by the
/// reading door and the read-write door alike; nothing is read through a
/// link, the file behind it keeps its bytes, and no open waits on a pipe.
#[test]
fn a_pipe_device_or_link_is_refused_by_name() {
    let dir = dir("kinds");
    let target = dir.join("target.bin");
    fs::write(&target, b"bytes behind a link").unwrap();
    let pipe = dir.join("pipe");
    mkfifo(&pipe);
    let link_to_file = dir.join("link-to-file");
    std::os::unix::fs::symlink(&target, &link_to_file).unwrap();
    let link_to_pipe = dir.join("link-to-pipe");
    std::os::unix::fs::symlink(&pipe, &link_to_pipe).unwrap();
    let dangling = dir.join("dangling");
    std::os::unix::fs::symlink(dir.join("nowhere"), &dangling).unwrap();
    let socket = dir.join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let folder = dir.join("folder");
    fs::create_dir(&folder).unwrap();

    let mut table: Vec<(PathBuf, Found)> = vec![
        (pipe.clone(), Found::Pipe),
        (link_to_file, Found::Symlink),
        (link_to_pipe, Found::Symlink),
        (dangling.clone(), Found::Symlink),
        (socket, Found::Socket),
        (folder, Found::Directory),
        (PathBuf::from("/dev/null"), Found::CharDevice),
    ];
    if let Some(device) = a_block_device() {
        table.push((device, Found::BlockDevice));
    }
    for (path, expected) in table {
        let asked = path.clone();
        let answered = answers_at_once(&pipe, move || both(&asked));
        for refusal in answered {
            let refusal = refusal.expect_err("refused");
            assert_eq!(refusal.word(), "read_not_regular", "{refusal}");
            assert_eq!(refusal.found(), Some(expected), "{refusal}");
            assert_eq!(refusal.path(), path, "{refusal}");
            assert!(!refusal.is_missing(), "{refusal}");
            let said = refusal.to_string();
            assert!(said.contains(expected.word()), "{said}");
            assert!(said.contains(expected.sentence()), "{said}");
            assert!(said.contains(&path.display().to_string()), "{said}");
            assert!(std::error::Error::source(&refusal).is_none());
        }
    }
    assert_eq!(fs::read(&target).unwrap(), b"bytes behind a link");
    assert!(
        !dir.join("nowhere").exists(),
        "a dangling link is not created through"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_absent_name_is_the_hosts_not_found_and_nothing_else_is() {
    let dir = dir("absent");
    let refusal = open_read(&dir.join("absent.bin")).unwrap_err();
    assert!(refusal.is_missing(), "{refusal}");
    assert_eq!((refusal.word(), refusal.found()), ("io", None));
    assert!(std::error::Error::source(&refusal).is_some());
    assert!(refusal.to_string().starts_with("io: open "), "{refusal}");
    assert!(
        !dir.join("absent.bin").exists(),
        "the read-write door creates nothing either"
    );
    assert!(
        open_read_write(&dir.join("absent.bin"))
            .unwrap_err()
            .is_missing()
    );
    assert!(!dir.join("absent.bin").exists());

    // A NAME BELOW A FILE is the host's refusal, kept as it is: neither
    // missing nor a kind.
    let file = dir.join("a-file");
    fs::write(&file, b"x").unwrap();
    let refusal = open_read(&file.join("below")).unwrap_err();
    assert_eq!((refusal.word(), refusal.found()), ("io", None), "{refusal}");
    let _ = fs::remove_dir_all(&dir);
}

/// A regular file the host will not open (no read permission) keeps the
/// host's own refusal: the look that names a kind finds a regular file and
/// names none. Skipped where the test runs with the power to read anyway.
#[test]
fn a_regular_file_the_host_refuses_keeps_the_hosts_words() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = dir("denied");
    let path = dir.join("denied.bin");
    fs::write(&path, b"x").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&path).is_err() {
        let refusal = open_read(&path).unwrap_err();
        assert_eq!((refusal.word(), refusal.found()), ("io", None), "{refusal}");
        assert!(!refusal.is_missing());
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(open_read_len(&path).unwrap().1, 1);
    let _ = fs::remove_dir_all(&dir);
}

/// **A WRITING VERB REFUSES A STORE WHOSE BARS FOLDER IS A LINK** (R3-16,
/// RA-298, FM-198): the shape `state/fix/shadow.sh` makes, `<root>/bars`
/// linked to another store's, is refused `linked_store_root` naming the link
/// and where it points; a store with its own `bars`, a new store with none,
/// and a root that is itself a link to a real store are not refused; a file
/// or a pipe in the place of `bars` is refused by what it is.
#[test]
fn a_writing_verb_refuses_a_store_whose_bars_folder_is_a_link() {
    let dir = dir("linked-root");
    let real = dir.join("real-store");
    fs::create_dir_all(real.join(BARS_DIR)).unwrap();
    fs::write(real.join(BARS_DIR).join("month.bin"), b"real month").unwrap();

    let shadow = dir.join("shadow");
    fs::create_dir_all(&shadow).unwrap();
    std::os::unix::fs::symlink(real.join(BARS_DIR), shadow.join(BARS_DIR)).unwrap();
    let refusal = refuse_linked_store_root(&shadow).unwrap_err();
    assert_eq!(refusal.word(), "linked_store_root");
    assert!(
        matches!(&refusal, StoreRootRefusal::Linked { root, bars, target }
            if *root == shadow && *bars == shadow.join(BARS_DIR)
                && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
        "{refusal:?}"
    );
    let said = refusal.to_string();
    assert!(said.starts_with("linked_store_root: "), "{said}");
    assert!(
        said.contains(&real.join(BARS_DIR).display().to_string()),
        "{said}"
    );
    assert!(std::error::Error::source(&refusal).is_none());
    assert_eq!(
        fs::read(real.join(BARS_DIR).join("month.bin")).unwrap(),
        b"real month"
    );

    // A DANGLING LINK is refused the same way: it is still not this store's.
    let dangling = dir.join("dangling-store");
    fs::create_dir_all(&dangling).unwrap();
    std::os::unix::fs::symlink(dir.join("nowhere"), dangling.join(BARS_DIR)).unwrap();
    assert_eq!(
        refuse_linked_store_root(&dangling).unwrap_err().word(),
        "linked_store_root"
    );

    // NOT REFUSED: its own bars, a new store, a root that is itself a link.
    refuse_linked_store_root(&real).unwrap();
    let new = dir.join("new-store");
    fs::create_dir_all(&new).unwrap();
    refuse_linked_store_root(&new).unwrap();
    let root_link = dir.join("root-link");
    std::os::unix::fs::symlink(&real, &root_link).unwrap();
    refuse_linked_store_root(&root_link).unwrap();

    // A FILE OR A PIPE where `bars` belongs is refused by what it is.
    let filed = dir.join("filed-store");
    fs::create_dir_all(&filed).unwrap();
    fs::write(filed.join(BARS_DIR), b"").unwrap();
    let refusal = refuse_linked_store_root(&filed).unwrap_err();
    assert_eq!(refusal.word(), "store_bars_not_a_directory");
    assert!(refusal.to_string().contains("a regular file"), "{refusal}");
    let piped = dir.join("piped-store");
    fs::create_dir_all(&piped).unwrap();
    mkfifo(&piped.join(BARS_DIR));
    let refusal = refuse_linked_store_root(&piped).unwrap_err();
    assert_eq!(refusal.word(), "store_bars_not_a_directory");
    assert!(refusal.to_string().contains("a pipe (FIFO)"), "{refusal}");

    // A ROOT THE HOST CANNOT LOOK INTO is refused, never taken for new.
    let under_a_file = dir.join("a-file");
    fs::write(&under_a_file, b"").unwrap();
    let refusal = refuse_linked_store_root(&under_a_file).unwrap_err();
    assert_eq!(refusal.word(), "io", "{refusal}");
    assert!(std::error::Error::source(&refusal).is_some());
    assert!(
        refusal.to_string().contains("cannot tell what"),
        "{refusal}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// **FM-198: A WRITER REFUSES A LINKED BARS DIRECTORY** (R3-16, RA-298), at
/// the month writer itself and not only at the verbs' door: a shadow whose
/// `bars` links to another store's is refused by `BarFile::open_or_create`
/// for a NIFTY spot month of each feed (GDFL 1s, Zerodha 1min), and for a
/// BANKNIFTY and an F&O stock month, by name (`symlinked`, naming
/// `<root>/bars`), before any directory, lock or month file is made through
/// the link; the same month on the store that owns its `bars` is written.
#[test]
fn a_writer_refuses_a_linked_bars_directory() {
    use brutex_core::vendor::Vendor;

    use crate::file::{BarFile, StoreError};
    use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    let dir = dir("linked-writer");
    let real = dir.join("real-store");
    fs::create_dir_all(real.join(BARS_DIR)).unwrap();
    let shadow = dir.join("shadow");
    fs::create_dir_all(&shadow).unwrap();
    std::os::unix::fs::symlink(real.join(BARS_DIR), shadow.join(BARS_DIR)).unwrap();
    let months = [
        (Vendor::Gdfl, "INDEX", "NIFTY", Timeframe::SECOND_1),
        (Vendor::Zerodha, "INDEX", "NIFTY", Timeframe::MINUTE_1),
        (Vendor::Gdfl, "INDEX", "BANKNIFTY", Timeframe::SECOND_1),
        (Vendor::Zerodha, "EQ", "RELIANCE", Timeframe::MINUTE_1),
    ];
    for (vendor, segment, symbol, timeframe) in months {
        let path = StorePath::new(PathParts {
            vendor,
            exchange: "NSE",
            segment,
            symbol,
            contract: None,
            timeframe,
            month: YearMonth::new(2019, 6).unwrap(),
            file: FileKind::Bars,
        })
        .unwrap();
        match BarFile::open_or_create(&shadow, path, 7) {
            Err(StoreError::Symlinked { path: at }) => {
                assert_eq!(at, shadow.join(BARS_DIR), "{symbol}: names the link");
            }
            Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
            Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
        }
        assert_eq!(
            fs::read_dir(real.join(BARS_DIR)).unwrap().count(),
            0,
            "{symbol}: nothing was made through the link"
        );
    }
    // THE SAME MONTH on a store that owns its bars is written: the refusal
    // above is the link's.
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2019, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    drop(BarFile::open_or_create(&real, path, 7).expect("its own bars are written"));
    assert_eq!(fs::read_dir(real.join(BARS_DIR)).unwrap().count(), 1);
    let _ = fs::remove_dir_all(&dir);
}
thread_local! {
    /// The `lstat`s `name_the_refusal` made on this thread.
    static LOOKS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Counts one look of `name_the_refusal` on this thread.
pub(super) fn count_look() {
    LOOKS.with(|looks| looks.set(looks.get() + 1));
}

/// The looks counted on this thread since the last call, and the count
/// cleared.
fn looks() -> u32 {
    LOOKS.with(|looks| looks.replace(0))
}

/// **AN ABSENT NAME IS REFUSED WITHOUT A LOOK; ONLY A NAME THE HOST WOULD
/// NOT OPEN IS LOOKED AT, ONCE.**
///
/// Kills `replace match guard why.kind() == ErrorKind::NotFound with false`
/// in `open_regular`: `ENOENT` would go through `name_the_refusal`, whose
/// `lstat` sees the same absence and says the same `io` refusal, so the
/// answer cannot tell them apart; the cost can, and the module states it (one
/// `lstat`, on the path of a refusal other than an absent name). Asked of an
/// absent name (no look), a final symbolic link (`ELOOP`: one look, which
/// names it), and a regular file opened (no look).
#[test]
fn an_absent_name_is_refused_without_a_look_and_a_link_with_one() {
    let dir = dir("looks");
    let _ = looks();
    let absent = open_read(&dir.join("absent")).unwrap_err();
    assert!(
        matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
        "{absent}"
    );
    assert_eq!(looks(), 0, "an absent name costs no look");
    let file = dir.join("file");
    fs::write(&file, b"x").unwrap();
    let link = dir.join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    let linked = open_read(&link).unwrap_err();
    assert_eq!(linked.found(), Some(Found::Symlink), "{linked}");
    assert_eq!(looks(), 1, "a refused open costs one look");
    open_read(&file).unwrap();
    assert_eq!(looks(), 0, "an open costs none");
    let _ = fs::remove_dir_all(&dir);
}
