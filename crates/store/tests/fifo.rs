//! **The read door never waits on a file that is not a file** — AC-whp-cx-0.
//!
//! `BarFile::open_existing` opened the bars, the `.lock` and the `.crc` with a
//! plain blocking `open(2)`. A FIFO at any of those three names (or a symlink
//! to one) parks the caller in the kernel until some writer opens the other
//! end: a `sweep-stored`, an HTTP worker or a pull hangs with no refusal, no
//! timeout and no log line. `catalog::walk` handed every non-directory entry to
//! `classify`, so a FIFO spelled like a month was listed as a held month and
//! then hung the first reader to open it.
//!
//! Every test here makes a REAL FIFO with `mkfifo` (test-only, as
//! `checksum_audit_tests.rs` already does) and runs the open on a worker thread
//! bounded by a timeout. On a timeout the test releases the blocked reader by
//! opening the FIFO's write end itself, so a regression fails loudly instead of
//! hanging the suite.
#![cfg(unix)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use brutex_core::vendor::Vendor;
use store::catalog;
use store::file::{Action, BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// How long an open may take before it is judged to be waiting on a peer.
const BOUND: Duration = Duration::from_secs(3);

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-fifo-{tag}-{}-{serial}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("a scratch root");
        Self { root }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("a month"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn bar(index: i64) -> Bar {
    Bar {
        // Inside the 2024-06 month the path names, on the minute grid: the
        // writer admits no other stamp since D-0915.
        ts_micros: 1_717_386_300_000_000 + index * 60_000_000,
        open: 100,
        high: 120,
        low: 90,
        close: 110,
        volume: 50,
        open_interest: OI_NULL,
    }
}

/// A sealed month with `rows` committed records, so all three siblings exist.
fn real_month(root: &Path, rows: i64) {
    let mut file = BarFile::open_or_create(root, month(), 7).expect("fixture month");
    if rows > 0 {
        let batch: Vec<Bar> = (0..rows).map(bar).collect();
        file.append(&batch).expect("fixture rows");
    }
}

fn mkfifo(at: &Path) {
    assert!(
        std::process::Command::new("mkfifo")
            .arg(at)
            .status()
            .expect("mkfifo runs")
            .success(),
        "mkfifo made {}",
        at.display()
    );
}

/// `open_existing` on a worker thread, bounded. `release` is the FIFO the
/// worker may be parked on; on a timeout its write end is opened here, which is
/// what wakes a blocked reader, and the test fails naming the hang.
fn bounded_open(root: &Path, release: &Path) -> Result<BarFile, StoreError> {
    let (send, recv) = mpsc::channel();
    let owned = root.to_path_buf();
    let worker = std::thread::spawn(move || {
        drop(send.send(BarFile::open_existing(&owned, month(), 7)));
    });
    if let Ok(outcome) = recv.recv_timeout(BOUND) {
        worker.join().expect("the worker returned");
        return outcome;
    }
    // Wake the parked reader so the suite does not hang, then fail.
    drop(fs::OpenOptions::new().write(true).open(release));
    drop(worker.join());
    panic!(
        "open_existing blocked for {BOUND:?} on the FIFO at {}",
        release.display()
    );
}

/// A FIFO at each of the three names is refused promptly, by name, with the
/// exact path and the action, and a rerun refuses identically.
#[test]
fn a_fifo_at_any_sibling_is_refused_promptly_and_by_name() {
    for kind in [FileKind::Bars, FileKind::Lock, FileKind::Checksums] {
        let scratch = Scratch::new(&format!("{kind:?}"));
        real_month(&scratch.root, 3);
        let at = month().with_file(kind).to_path_buf(&scratch.root);
        assert!(at.is_file(), "{kind:?}: the fixture made the sibling");
        fs::remove_file(&at).expect("remove the real sibling");
        mkfifo(&at);

        let expected = StoreError::NotARegularFile {
            path: at.clone(),
            action: Action::Open,
        };
        let first = bounded_open(&scratch.root, &at).err();
        assert_eq!(first, Some(expected.clone()), "{kind:?}");
        // Reruns are safe and say the same thing, byte for byte.
        let again = bounded_open(&scratch.root, &at).err();
        assert_eq!(again, Some(expected.clone()), "{kind:?} rerun");
        let rendered = expected.to_string();
        assert!(
            rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
            "{kind:?}: the refusal names the path: {rendered}"
        );
        // The FIFO is left where it was: the read door removes nothing.
        assert!(
            fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
            "{kind:?}: the FIFO is still there"
        );
    }
}

/// A FIFO with no committed rows behind it (the empty-month boundary) is
/// refused the same way: the guard is on the open, not on the header.
#[test]
fn a_fifo_beside_an_empty_month_is_refused() {
    let scratch = Scratch::new("empty");
    real_month(&scratch.root, 0);
    let lock = month().with_file(FileKind::Lock).to_path_buf(&scratch.root);
    fs::remove_file(&lock).expect("remove lock");
    mkfifo(&lock);
    assert_eq!(
        bounded_open(&scratch.root, &lock).err(),
        Some(StoreError::NotARegularFile {
            path: lock,
            action: Action::Open,
        })
    );
}

/// A symlink to a FIFO is followed and then refused on what it reaches; the
/// error names the path the store asked for, not the target.
#[test]
fn a_symlink_to_a_fifo_is_refused() {
    let scratch = Scratch::new("link");
    real_month(&scratch.root, 2);
    let bars = month().to_path_buf(&scratch.root);
    let target = scratch.root.join("elsewhere.fifo");
    mkfifo(&target);
    fs::remove_file(&bars).expect("remove bars");
    std::os::unix::fs::symlink(&target, &bars).expect("symlink");
    assert_eq!(
        bounded_open(&scratch.root, &target).err(),
        Some(StoreError::NotARegularFile {
            path: bars,
            action: Action::Open,
        })
    );
}

/// A character device is the other non-regular file `open(2)` happily returns.
/// `/dev/null` reads as zero bytes; it must be refused as not a regular file,
/// never parsed as an empty month.
#[test]
fn a_character_device_at_the_lock_is_refused() {
    let null = Path::new("/dev/null");
    if !null.exists() {
        return;
    }
    let scratch = Scratch::new("chardev");
    real_month(&scratch.root, 1);
    let lock = month().with_file(FileKind::Lock).to_path_buf(&scratch.root);
    fs::remove_file(&lock).expect("remove lock");
    std::os::unix::fs::symlink(null, &lock).expect("symlink");
    assert_eq!(
        BarFile::open_existing(&scratch.root, month(), 7).err(),
        Some(StoreError::NotARegularFile {
            path: lock,
            action: Action::Open,
        })
    );
}

/// A directory at a read sibling keeps its existing, more specific name.
#[test]
fn a_directory_at_the_lock_is_named_as_a_directory() {
    let scratch = Scratch::new("lockdir");
    real_month(&scratch.root, 1);
    let lock = month().with_file(FileKind::Lock).to_path_buf(&scratch.root);
    fs::remove_file(&lock).expect("remove lock");
    fs::create_dir(&lock).expect("directory masquerading as lock");
    assert_eq!(
        BarFile::open_existing(&scratch.root, month(), 7).err(),
        Some(StoreError::IsADirectory {
            path: lock,
            action: Action::Open,
        })
    );
}

/// No regression: a regular month, and a symlink to a regular bar file, still
/// open and read back every row, first and last.
#[test]
fn regular_files_and_symlinks_to_them_still_open() {
    let scratch = Scratch::new("regular");
    real_month(&scratch.root, 5);
    let file = BarFile::open_existing(&scratch.root, month(), 7).expect("regular month");
    assert_eq!(file.records(), 5);
    assert_eq!(file.read_record(0).expect("first"), bar(0));
    assert_eq!(file.read_record(4).expect("last"), bar(4));
    drop(file);

    let bars = month().to_path_buf(&scratch.root);
    let moved = scratch.root.join("real.bin");
    fs::rename(&bars, &moved).expect("move");
    std::os::unix::fs::symlink(&moved, &bars).expect("symlink");
    let file = BarFile::open_existing(&scratch.root, month(), 7).expect("symlinked month");
    assert_eq!(file.records(), 5);
    assert_eq!(file.read_record(4).expect("last"), bar(4));
}

/// An absent lock is created and held shared, so a writer that arrives while
/// the reader is live is refused (store1-1, D-2551); an absent bar file is
/// still `Missing`.
#[test]
fn absent_siblings_keep_their_answers() {
    let scratch = Scratch::new("absent");
    real_month(&scratch.root, 1);
    let lock = month().with_file(FileKind::Lock).to_path_buf(&scratch.root);
    fs::remove_file(&lock).expect("remove lock");
    let file = BarFile::open_existing(&scratch.root, month(), 7).expect("no lock is fine");
    assert_eq!(file.records(), 1);
    assert!(lock.is_file(), "the reader created the absent lock");
    assert_eq!(
        BarFile::open_or_create(&scratch.root, month(), 7).err(),
        Some(StoreError::ReaderHolds { path: lock.clone() }),
        "a later writer is excluded while the lockless reader is live, and is \
         told a reader holds the month (D-2552)"
    );
    drop(file);
    assert_eq!(
        BarFile::open_or_create(&scratch.root, month(), 7)
            .expect("the month is free again")
            .records(),
        1
    );
    let bars = month().to_path_buf(&scratch.root);
    fs::remove_file(&bars).expect("remove bars");
    assert_eq!(
        BarFile::open_existing(&scratch.root, month(), 7).err(),
        Some(StoreError::Missing {
            path: bars,
            action: Action::Open,
        })
    );
}

/// The catalog does not list a FIFO spelled like a month as held. It is
/// counted in its own bucket, the census reconciles, and a real month beside it
/// is still found.
#[test]
fn the_catalog_counts_a_fifo_month_and_does_not_hold_it() {
    let scratch = Scratch::new("catalog");
    real_month(&scratch.root, 1);
    let dir = month()
        .to_path_buf(&scratch.root)
        .parent()
        .expect("month dir")
        .to_path_buf();
    mkfifo(&dir.join("2024-07.bin"));
    std::os::unix::fs::symlink(dir.join("2024-07.bin"), dir.join("2024-08.bin")).expect("symlink");

    let out = catalog::walk(&scratch.root).expect("the walk runs");
    let months: Vec<String> = out.held.iter().map(|h| h.month.to_string()).collect();
    assert_eq!(months, vec!["2024-06".to_owned()], "only the real month");
    // D-0766 never follows a link below `bars/`, so the link is `linked`
    // and only the FIFO itself is `not_regular`.
    assert_eq!(out.census.not_regular, 1, "the FIFO");
    assert_eq!(out.census.linked, 1, "the link to it");
    assert_eq!(out.census.spot, 1);
    assert_eq!(out.census.other_kind, 3, "the real .lock, .crc and .tix");
    assert_eq!(out.census.seen, 6);
    assert!(out.census.reconciles());
}
