#![cfg(test)]
#![allow(clippy::expect_used, clippy::panic)]
//! A refused release of the shared lock `repair` takes is the caller's
//! `RepairError::Io`, with the host's kind and the visibility the moment
//! deserves. D-0693.
//!
//! `store/tests/repair.rs` proves the protocol, and as an integration test it
//! cannot reach the `cfg(test)` seam in `crate::flock::tests`, the only way to
//! make an unlock refuse. So the two release branches were driven by no test,
//! and `publication_may_be_visible` flipped on either, or either release
//! replaced by a plain drop (whose `Drop` unlocks and only logs), left every
//! store test green. These two drive each branch through the public call.
//!
//! Each then asks for the refused lock exclusively, on a fresh open file
//! description. A re-read or a retry takes the lock shared, and a shared lock
//! is granted beside one that a refused release left held on a leaked
//! descriptor, so neither can show that the refusal took nothing with it.
//!
//! Each holds `crate::emits::hold_the_sink()`: `publish` appends, and an
//! append emits `store.append` into whatever sink the binary installed.

use std::fs::{self, File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::{Published, RepairError, Revision, RevisionReader, publish};
use crate::file::BarFile;
use crate::flock::Flock;
use crate::flock::tests::refuse_unlock;
use crate::format::{Bar, OI_NULL};
use crate::header::Header;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A scratch store root holding one sealed source month, removed on drop.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-store-repair-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch store root");
        Self { root }
    }

    /// Commits `rows` as the source month and returns its header.
    fn source(&self, rows: &[Bar]) -> Header {
        let mut writer =
            BarFile::open_or_create(&self.root, path(), SYMBOL).expect("the source month opens");
        writer.append(rows).expect("the source rows commit");
        writer.header()
    }

    fn publish(&self, expected: Header, merged: &[Bar]) -> Result<Published, RepairError> {
        publish(&self.root, path(), SYMBOL, revision(), expected, merged)
    }

    fn read(&self) -> Result<RevisionReader, RepairError> {
        RevisionReader::open(&self.root, path(), SYMBOL, revision())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.root);
    }
}

/// The symbol id the source and the revision are committed under.
const SYMBOL: u32 = 7;

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("2024-06"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn revision() -> Revision {
    Revision::new(1).expect("ordinal 1")
}

/// Whether another holder could take `lock` exclusively now.
///
/// A fresh open file description asks for the exclusive lock without
/// waiting, and releases what it took. A shared holder refuses an exclusive
/// request as an exclusive holder does, so a refused release whose lock
/// outlived the call, on a descriptor still open anywhere, makes this
/// `false`. A shared request would be granted beside that shared lock, which
/// is why a re-read cannot show it.
fn free(lock: &Path) -> bool {
    let file = File::open(lock).expect("the lock file opens");
    match Flock::try_lock(file, lock) {
        Ok(held) => {
            held.release().expect("the probe releases what it took");
            true
        }
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
    }
}

/// The `index`-th one-minute bar of 2024-06-03, in paisa.
fn bar(index: i64) -> Bar {
    Bar {
        ts_micros: 1_717_386_300_000_000 + index * 60_000_000,
        open: 100,
        high: 110,
        low: 90,
        close: 105,
        volume: index,
        open_interest: OI_NULL,
    }
}

/// **A REFUSED SOURCE RELEASE AFTER A WRITTEN REVISION SAYS THE PUBLICATION
/// MAY BE VISIBLE, AND IT IS.**
///
/// `publish` writes the revision and its receipt, and only then releases the
/// source's shared lock. `RepairError::Io`'s contract is that `true` means a
/// caller must not report that publication did not happen, so a refusal at
/// that point says `true`. The revision is then read back whole, and a retry
/// of the same request is `Reused`, which is what "may be visible" promised.
///
/// Two unlocks are let through first: the revision writer's month lock, which
/// `write_revision` drops once the receipt is synced, and the reservation's
/// own exclusive lock it then releases (D-2551), both before the source
/// release. If that order changes, the refusal lands elsewhere, and this test
/// fails on the error it compares rather than passing by accident.
#[test]
fn a_refused_source_release_after_the_write_says_the_publication_may_be_visible() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("publish");
    let expected = fixture.source(&[bar(1), bar(3)]);
    let merged = [bar(0), bar(1), bar(2), bar(3)];

    refuse_unlock(2, io::ErrorKind::PermissionDenied);
    let refused = fixture
        .publish(expected, &merged)
        .expect_err("the source lock's release was refused and publish must say so");
    let source_lock = path().with_file(FileKind::Lock).to_path_buf(&fixture.root);
    assert_eq!(
        refused,
        RepairError::Io {
            path: source_lock.clone(),
            operation: "release shared lock",
            kind: io::ErrorKind::PermissionDenied,
            publication_may_be_visible: true,
        },
        "the source release's refusal, with the host's kind, after the revision was written"
    );
    assert!(
        free(&source_lock),
        "the refusal kept the source lock: its guard's own descriptor must close with it"
    );

    let revised = fixture
        .read()
        .expect("the revision the refusal said may be visible is complete");
    assert_eq!(revised.source_header(), expected);
    assert_eq!(revised.header().n_valid, 4);
    for (index, row) in (0_u64..).zip(&merged) {
        assert_eq!(
            revised.read_record(index).expect("a written row"),
            *row,
            "row {index} of the revision"
        );
    }
    drop(revised);
    assert_eq!(
        fixture.publish(expected, &merged),
        Ok(Published::Reused),
        "the same request again finds the publication it was told may exist"
    );
}

/// **A REFUSED RELEASE WHEN A REVISION IS OPENED SAYS NOTHING WAS PUBLISHED.**
///
/// `RevisionReader::open` only reads, so a refusal of its shared lock's
/// release says `false`, with the host's kind. It is the first unlock the open
/// makes, so none is let through.
#[test]
fn a_refused_release_when_a_revision_is_opened_says_nothing_was_published() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("open");
    let expected = fixture.source(&[bar(1), bar(3)]);
    let merged = [bar(0), bar(1), bar(2), bar(3)];
    assert_eq!(
        fixture.publish(expected, &merged),
        Ok(Published::Created),
        "the premise: a revision to open"
    );

    refuse_unlock(0, io::ErrorKind::PermissionDenied);
    let refused = fixture
        .read()
        .expect_err("the revision lock's release was refused and open must say so");
    let revision_root = fixture.root.join("bar-revisions-v1").join("1");
    let revision_lock = path().with_file(FileKind::Lock).to_path_buf(&revision_root);
    assert_eq!(
        refused,
        RepairError::Io {
            path: revision_lock.clone(),
            operation: "release shared lock",
            kind: io::ErrorKind::PermissionDenied,
            publication_may_be_visible: false,
        },
        "the revision lock's refusal, with the host's kind, on a read"
    );
    assert!(
        free(&revision_lock),
        "the refusal kept the revision lock: its guard's own descriptor must close with it"
    );
    fixture
        .read()
        .expect("the revision reads once the refused open has returned");
}

#[test]
fn repair_refusals_name_the_reason_path_and_publication_visibility() {
    let at = PathBuf::from("revision/2024-06.bin");
    let cases = [
        (
            RepairError::RevisionLimit,
            "repair revision ordinal outside 1..=1024",
        ),
        (
            RepairError::RowLimit,
            "repair exceeds the 100000-row ceiling",
        ),
        (
            RepairError::StaleSource,
            "repair source header changed; merge again",
        ),
        (
            RepairError::MissingTimestamp(17),
            "repair omits source timestamp 17",
        ),
        (
            RepairError::UnsealedSource,
            "repair requires checksum-protected source bars",
        ),
        (
            RepairError::Conflict,
            "repair revision already holds a different request",
        ),
        (
            RepairError::Incomplete(at.clone()),
            "incomplete repair at revision/2024-06.bin; preserved",
        ),
        (
            RepairError::InvalidReceipt(at.clone()),
            "invalid repair receipt at revision/2024-06.bin",
        ),
        (
            RepairError::Busy(at.clone()),
            "repair revision revision/2024-06.bin is being published by another caller; retry the same ordinal",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
    let store = crate::file::StoreError::NotCommitted {
        index: 7,
        n_valid: 3,
    };
    assert_eq!(
        RepairError::from(store).to_string(),
        "record 7 is past the 3 committed"
    );
    for visible in [false, true] {
        let error = RepairError::Io {
            path: at.clone(),
            operation: "sync receipt",
            kind: io::ErrorKind::PermissionDenied,
            publication_may_be_visible: visible,
        };
        assert_eq!(
            error.to_string(),
            format!(
                "sync receipt revision/2024-06.bin: PermissionDenied; publication may be visible: {visible}"
            )
        );
    }
}

#[test]
fn a_receipt_for_another_source_symbol_or_timeframe_is_refused_without_writes() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for foreign_symbol in [true, false] {
        let fixture = Fixture::new("foreign-receipt");
        let expected = fixture.source(&[bar(1)]);
        assert_eq!(fixture.publish(expected, &[bar(1)]), Ok(Published::Created));
        let receipt = path()
            .to_path_buf(&revision().root(&fixture.root))
            .with_extension("repair-v1");
        let mut bytes = fs::read(&receipt).expect("published receipt");
        let mut foreign = expected;
        if foreign_symbol {
            foreign.symbol_id = SYMBOL + 1;
        } else {
            foreign.timeframe_secs = 300;
        }
        bytes
            .get_mut(16..80)
            .expect("source header")
            .copy_from_slice(&foreign.commit().expect("well-formed foreign header").bytes);
        fs::write(&receipt, &bytes).expect("foreign source receipt");
        assert_eq!(
            fixture.read().expect_err("foreign source identity"),
            RepairError::InvalidReceipt(receipt.clone())
        );
        assert_eq!(
            fs::read(&receipt).expect("refusal preserves receipt"),
            bytes
        );
    }
}

#[test]
fn a_checksum_valid_source_with_nonincreasing_timestamps_is_not_repaired() {
    use std::os::unix::fs::FileExt;

    let _sink_is_mine = crate::emits::hold_the_sink();
    for middle in [bar(0), bar(-1)] {
        let fixture = Fixture::new("unordered-source");
        let ordered = [bar(0), bar(2), bar(4)];
        let expected = fixture.source(&ordered);
        let physical = path().to_path_buf(&fixture.root);
        let checksums = path()
            .with_file(FileKind::Checksums)
            .to_path_buf(&fixture.root);
        // Model a checksum-valid historical producer that did not enforce
        // today's batch ordering. Keep the header's first/last stamps valid.
        let mut middle = middle;
        middle.volume = 0;
        let rows = [ordered[0], middle, ordered[2]];
        let bytes: Vec<u8> = rows.iter().flat_map(Bar::image).collect();
        let layout = crate::layout::Layout::V2;
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&physical)
            .expect("source to seed");
        assert_eq!(
            file.write_at(&bytes, layout.offset_of(0).expect("first record"))
                .expect("seed records"),
            bytes.len()
        );
        let checksum = crate::block::seal(layout, 3, 0, &bytes).expect("valid block geometry");
        fs::write(&checksums, checksum.to_le_bytes()).expect("matching CRC");
        let before = fs::read(&physical).expect("source bytes");
        assert_eq!(
            fixture.publish(expected, &ordered),
            Err(RepairError::Store(
                crate::file::StoreError::BatchNotOrdered {
                    at: 1,
                    previous: rows[0].ts_micros,
                    next: middle.ts_micros,
                }
            ))
        );
        assert_eq!(fs::read(&physical).expect("unchanged source"), before);
        assert_eq!(
            fs::read(&checksums).expect("unchanged CRC"),
            checksum.to_le_bytes()
        );
        assert!(
            !revision().root(&fixture.root).exists(),
            "refuse before reservation"
        );
    }
}

/// store2-1, D-2551: while another caller holds an ordinal's reservation,
/// the same request is `Busy` (retry the same ordinal), never an I/O failure
/// that says nothing was published and never `Incomplete`; once that holder
/// is gone, a reservation without a receipt is `Incomplete` as before.
#[test]
fn a_live_publisher_of_the_same_ordinal_is_busy_not_abandoned() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("busy");
    let expected = fixture.source(&[bar(1)]);
    let physical = path().to_path_buf(&revision().root(&fixture.root));
    let reservation = physical.with_extension("reserved-v1");
    fs::create_dir_all(reservation.parent().expect("month directory")).expect("revision directory");
    let held = super::reserve(&reservation).expect("a live publisher's reservation");
    assert_eq!(
        super::reserve(&reservation).err(),
        Some(RepairError::Busy(reservation.clone())),
        "a lost reservation race"
    );
    assert_eq!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Busy(reservation.clone())),
        "a retry during a live publication"
    );
    held.release().expect("the publisher stops");
    assert_eq!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Incomplete(
            physical.with_extension("repair-v1")
        ))
    );
}

#[test]
fn every_unexplained_revision_sibling_and_receipt_is_preserved() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for kind in [
        Some(FileKind::Bars),
        Some(FileKind::Checksums),
        Some(FileKind::Lock),
        None,
    ] {
        let fixture = Fixture::new("unexplained-sibling");
        let expected = fixture.source(&[bar(1)]);
        let root = revision().root(&fixture.root);
        let physical = path().to_path_buf(&root);
        let orphan = kind.map_or_else(
            || physical.with_extension("repair-v1"),
            |kind| path().with_file(kind).to_path_buf(&root),
        );
        fs::create_dir_all(orphan.parent().expect("month directory")).expect("revision directory");
        fs::write(&orphan, b"unexplained evidence").expect("orphan file");
        assert_eq!(
            fixture.publish(expected, &[bar(1)]),
            Err(RepairError::Incomplete(orphan.clone()))
        );
        assert_eq!(
            fs::read(&orphan).expect("orphan survives"),
            b"unexplained evidence"
        );
        assert!(
            physical.with_extension("reserved-v1").is_file(),
            "failed reservation stays visible"
        );
    }
}

#[test]
fn an_uninspectable_revision_path_is_a_named_nonpublication_refusal() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("inspect-error");
    let expected = fixture.source(&[bar(1)]);
    let obstacle = fixture.root.join("bar-revisions-v1");
    fs::write(&obstacle, b"not a directory").expect("obstruct the revision path");
    let reservation = path()
        .to_path_buf(&revision().root(&fixture.root))
        .with_extension("reserved-v1");
    assert_eq!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Io {
            path: reservation,
            operation: "inspect revision",
            kind: io::ErrorKind::NotADirectory,
            publication_may_be_visible: false,
        })
    );
    assert_eq!(
        fs::read(obstacle).expect("obstacle remains"),
        b"not a directory"
    );
}

#[test]
fn an_unopenable_receipt_is_named_and_never_replaced_on_read_or_retry() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("receipt-open-error");
    let rows = [bar(1)];
    let expected = fixture.source(&rows);
    assert_eq!(fixture.publish(expected, &rows), Ok(Published::Created));
    let physical = path().to_path_buf(&revision().root(&fixture.root));
    let receipt = physical.with_extension("repair-v1");
    let saved = fixture.root.join("saved-receipt");
    let receipt_bytes = fs::read(&receipt).expect("published receipt");
    let data_bytes = fs::read(&physical).expect("published revision");
    fs::rename(&receipt, &saved).expect("preserve the receipt");
    std::os::unix::fs::symlink(&receipt, &receipt).expect("unopenable receipt loop");
    let host = File::open(&receipt).expect_err("loop is an open error");
    assert_ne!(host.kind(), io::ErrorKind::NotFound);
    let refusal = RepairError::Io {
        path: receipt.clone(),
        operation: "open receipt",
        kind: host.kind(),
        publication_may_be_visible: false,
    };
    assert_eq!(fixture.read().expect_err("cannot read receipt"), refusal);
    assert_eq!(fixture.publish(expected, &rows), Err(refusal));
    assert_eq!(fs::read_link(&receipt).expect("loop preserved"), receipt);
    assert_eq!(
        fs::read(&saved).expect("original receipt preserved"),
        receipt_bytes
    );
    assert_eq!(fs::read(&physical).expect("revision preserved"), data_bytes);
    fs::remove_file(&receipt).expect("remove injected fault");
    fs::rename(&saved, &receipt).expect("restore exact receipt");
    assert_eq!(fixture.publish(expected, &rows), Ok(Published::Reused));
    assert_eq!(
        fixture.read().expect("restored reader").read_record(0),
        Ok(rows[0])
    );
}

/// A real directory obstruction after preflight refuses before reservation.
/// This is private write-body proof; the test owns the source-lock lifetime.
#[test]
fn a_post_preflight_directory_obstruction_refuses_without_publication() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("post-preflight-directory");
    let expected = fixture.source(&[bar(1), bar(3)]);
    let merged = [bar(0), bar(1), bar(2), bar(3)];
    let original = [FileKind::Bars, FileKind::Checksums, FileKind::Lock].map(|kind| {
        let path = path().with_file(kind).to_path_buf(&fixture.root);
        let bytes = fs::read(&path).expect("original source image");
        (path, bytes)
    });
    let revision_root = revision().root(&fixture.root);
    let physical = path().to_path_buf(&revision_root);
    let directory = physical.parent().expect("revision month directory");
    let reservation = physical.with_extension("reserved-v1");
    super::survey(&merged).expect("valid merged batch");
    super::check_header(expected).expect("sealed bounded source");
    assert!(!super::exists(&reservation).expect("preflight can inspect the absent reservation"));
    let lock_path = path().with_file(FileKind::Lock).to_path_buf(&fixture.root);
    let held = super::shared_lock(lock_path.clone()).expect("preflight shared lock");
    let source = BarFile::open_existing(&fixture.root, path(), SYMBOL).expect("preflight source");
    assert_eq!(source.header(), expected);
    super::retain_timestamps(&source, &merged).expect("every source timestamp retained");
    assert!(!free(&lock_path), "actual shared source locks are held");

    fs::create_dir_all(revision_root.parent().expect("revision namespace"))
        .expect("prepare the namespace after preflight");
    fs::write(&revision_root, b"namespace obstruction").expect("file blocks a directory component");
    assert_eq!(
        super::write_revision(&fixture.root, path(), SYMBOL, revision(), expected, &merged),
        Err(RepairError::Io {
            path: directory.to_path_buf(),
            operation: "create revision directories",
            kind: io::ErrorKind::NotADirectory,
            publication_may_be_visible: false,
        })
    );
    assert_eq!(
        fs::read(&revision_root).expect("obstruction preserved"),
        b"namespace obstruction"
    );
    for output in [
        physical.clone(),
        path()
            .with_file(FileKind::Checksums)
            .to_path_buf(&revision_root),
        path().with_file(FileKind::Lock).to_path_buf(&revision_root),
        reservation,
        physical.with_extension("repair-v1"),
    ] {
        assert_eq!(
            fs::symlink_metadata(output)
                .expect_err("no output exists through the obstruction")
                .kind(),
            io::ErrorKind::NotADirectory
        );
    }
    for (path, bytes) in &original {
        assert_eq!(fs::read(path).expect("source preserved on refusal"), *bytes);
    }
    assert!(
        !free(&lock_path),
        "the private body does not release its caller's lock"
    );
    drop(source);
    assert!(
        !free(&lock_path),
        "the explicit preflight lock is still held"
    );
    held.release().expect("the test's lock owner releases");
    assert!(free(&lock_path), "no lock survives its owner");

    fs::remove_file(&revision_root).expect("remove only the injected obstruction");
    assert_eq!(fixture.publish(expected, &merged), Ok(Published::Created));
    let revised = fixture
        .read()
        .expect("ordinary public reader admits the revision");
    assert_eq!(revised.source_header(), expected);
    assert_eq!(revised.header().n_valid, 4);
    for (index, row) in (0_u64..).zip(&merged) {
        assert_eq!(
            revised.read_record(index).expect("complete revised row"),
            *row
        );
    }
    for (path, bytes) in &original {
        assert_eq!(fs::read(path).expect("source preserved on success"), *bytes);
    }
}
