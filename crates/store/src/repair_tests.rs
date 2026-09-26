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
/// One unlock is let through first: the revision writer's month lock, which
/// its guard's `Drop` releases as `write_revision` returns, before the source
/// release. If that order changes, the refusal lands elsewhere, and this test
/// fails on the error it compares rather than passing by accident.
#[test]
fn a_refused_source_release_after_the_write_says_the_publication_may_be_visible() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new("publish");
    let expected = fixture.source(&[bar(1), bar(3)]);
    let merged = [bar(0), bar(1), bar(2), bar(3)];

    refuse_unlock(1, io::ErrorKind::PermissionDenied);
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
