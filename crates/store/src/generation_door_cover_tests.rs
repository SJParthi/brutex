#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the generation pointer at its two edges no other test
//! reached: a pointer that already holds its last record refuses the next
//! promotion, and a non-directory inside the pointer tree refuses the read
//! by the pointer's path. Every bar is invented.

use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::format::{Bar, OI_NULL};
use crate::path::{PathParts, Timeframe, YearMonth};

const SYMBOL: u32 = 0x6E4;
const MINUTE: i64 = 60_000_000;

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let at = std::env::temp_dir().join(format!(
            "brutex-generation-door-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&at);
        fs::create_dir_all(&at).unwrap();
        Self(at)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "QGENC",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Invented one-minute bars at minutes `minutes` after 09:15 IST on 3 June
/// 2024.
fn bars(minutes: std::ops::Range<i64>) -> Vec<Bar> {
    let origin =
        YearMonth::new(2024, 6).unwrap().ist_bounds_micros().0 + (2 * 1_440 + 555) * MINUTE;
    minutes
        .map(|k| Bar {
            ts_micros: origin + k * MINUTE,
            open: 3_030_000 + k,
            high: 3_030_900,
            low: 3_029_100,
            close: 3_030_500,
            volume: 6,
            open_interest: OI_NULL,
        })
        .collect()
}

/// **A POINTER THAT HOLDS ITS LAST RECORD REFUSES THE NEXT PROMOTION.** The
/// month is served from revision 1, whose record is the pointer's last
/// possible one (record 1,023, the records before it unverifiable): a
/// published revision 2 is refused `RevisionLimit`, and revision 1 is still
/// served. The same pointer with room left promotes revision 2 (the control).
#[test]
fn a_pointer_that_holds_its_last_record_refuses_the_next_promotion() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("full");
    let held = bars(0..6);
    let original = {
        let mut writer = BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap();
        writer.append(&held).unwrap();
        writer.header()
    };
    let first = Revision::new(1).unwrap();
    repair::publish(&root.0, month(), SYMBOL, first, original, &held).unwrap();
    promote(&root.0, month(), SYMBOL, first).unwrap();
    let pointer_file = pointer_path(&root.0, month());
    let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
        panic!("revision 1 is served");
    };
    let served_header = BarFile::open_existing(&root.0, month(), SYMBOL)
        .unwrap()
        .header();
    let mut more = held.clone();
    more.extend(bars(9..10));
    let second = Revision::new(2).unwrap();
    repair::publish(&root.0, month(), SYMBOL, second, served_header, &more).unwrap();
    let last = Pointer {
        record: MAX_RECORDS - 1,
        ..record0
    };
    let mut image = vec![0_u8; usize::try_from((MAX_RECORDS - 1) * RECORD_LEN).unwrap()];
    image.extend_from_slice(&last.encode());
    fs::write(&pointer_file, &image).unwrap();
    assert_eq!(served(&root.0, month()).unwrap().ordinal(), 1);
    assert_eq!(
        promote(&root.0, month(), SYMBOL, second),
        Err(RepairError::RevisionLimit)
    );
    assert_eq!(served(&root.0, month()).unwrap().ordinal(), 1);
    assert_eq!(
        fs::read(&pointer_file).unwrap(),
        image,
        "nothing was written"
    );
    // The control: one record fewer leaves room for exactly one more.
    let spare = Pointer {
        record: MAX_RECORDS - 2,
        ..record0
    };
    let mut image = vec![0_u8; usize::try_from((MAX_RECORDS - 2) * RECORD_LEN).unwrap()];
    image.extend_from_slice(&spare.encode());
    fs::write(&pointer_file, &image).unwrap();
    assert!(matches!(
        promote(&root.0, month(), SYMBOL, second),
        Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
    ));
    assert_eq!(served(&root.0, month()).unwrap().ordinal(), 2);
}

/// **A NON-DIRECTORY INSIDE THE POINTER TREE REFUSES THE READ BY THE
/// POINTER'S PATH.** A store root that is not a directory serves the
/// original (its own open names that); a file where the pointer tree's own
/// directory goes, under a root that IS one, is refused, never read as "no
/// pointer".
#[test]
fn a_non_directory_inside_the_pointer_tree_refuses_the_read() {
    let root = Root::new("tree-file");
    fs::write(root.0.join(ROOT), b"not a directory").unwrap();
    match served(&root.0, month()) {
        Err(StoreError::Generation { path, reason }) => {
            assert_eq!(path, pointer_path(&root.0, month()));
            assert!(reason.starts_with("cannot open it:"), "{reason}");
        }
        other => panic!("the pointer read is refused: {other:?}"),
    }
    // The control: a root that is itself a file serves the original.
    let file_root = root.0.join("a-file-root");
    fs::write(&file_root, b"a file").unwrap();
    assert!(matches!(served(&file_root, month()), Ok(Served::Original)));
}

/// **A LINK AT THE POINTER'S NAME REFUSES THE READ BY THE POINTER'S PATH.**
/// The pointer is opened without following a link, so a linked pointer is
/// refused in the host's words, never followed and never read as "no
/// pointer".
#[test]
fn a_linked_pointer_refuses_the_read() {
    let root = Root::new("linked");
    let pointer_file = pointer_path(&root.0, month());
    fs::create_dir_all(pointer_file.parent().unwrap()).unwrap();
    let elsewhere = root.0.join("elsewhere.ptr");
    fs::write(&elsewhere, b"").unwrap();
    std::os::unix::fs::symlink(&elsewhere, &pointer_file).unwrap();
    match served(&root.0, month()) {
        Err(StoreError::Generation { path, reason }) => {
            assert_eq!(path, pointer_file);
            assert!(reason.starts_with("cannot open it:"), "{reason}");
        }
        other => panic!("the linked pointer is refused: {other:?}"),
    }
}

/// **A PROMOTION THE POINTER'S OWN PATH REFUSES IS REFUSED BY THAT STEP,
/// AND THE ORIGINAL IS STILL SERVED.** A file where the pointer's directory
/// goes refuses the directory's creation; a directory where the pointer file
/// goes refuses its open; a lock the host refuses (the flock test seam)
/// refuses the lock. Each names its path and step and claims nothing
/// visible; with the path made good the same promotion lands.
#[test]
fn a_promotion_the_pointers_path_refuses_is_refused_by_that_step() {
    let root = Root::new("pointer-refusals");
    let held = bars(0..6);
    let original = {
        let mut writer = BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap();
        writer.append(&held).unwrap();
        writer.header()
    };
    let first = Revision::new(1).unwrap();
    repair::publish(&root.0, month(), SYMBOL, first, original, &held).unwrap();
    let pointer_file = pointer_path(&root.0, month());
    let directory = pointer_file.parent().unwrap().to_path_buf();
    fs::create_dir_all(directory.parent().unwrap()).unwrap();
    fs::write(&directory, b"not a directory").unwrap();
    match promote(&root.0, month(), SYMBOL, first) {
        Err(RepairError::Io {
            path,
            operation: "create pointer directories",
            publication_may_be_visible: false,
            ..
        }) => assert_eq!(path, directory),
        other => panic!("the directory's creation is refused: {other:?}"),
    }
    fs::remove_file(&directory).unwrap();
    fs::create_dir_all(&pointer_file).unwrap();
    match promote(&root.0, month(), SYMBOL, first) {
        Err(RepairError::Io {
            path,
            operation: "open pointer",
            publication_may_be_visible: false,
            ..
        }) => assert_eq!(path, pointer_file),
        other => panic!("the pointer's open is refused: {other:?}"),
    }
    fs::remove_dir(&pointer_file).unwrap();
    crate::flock::tests::refuse_next_lock(std::io::ErrorKind::Interrupted);
    match promote(&root.0, month(), SYMBOL, first) {
        Err(RepairError::Io {
            path,
            operation: "lock pointer",
            kind: std::io::ErrorKind::Interrupted,
            publication_may_be_visible: false,
        }) => assert_eq!(path, pointer_file),
        other => panic!("the pointer's lock is refused: {other:?}"),
    }
    assert!(!crate::flock::tests::lock_refusal_armed());
    assert!(matches!(served(&root.0, month()), Ok(Served::Original)));
    assert!(matches!(
        promote(&root.0, month(), SYMBOL, first),
        Ok(Promoted::Promoted(_))
    ));
    assert_eq!(served(&root.0, month()).unwrap().ordinal(), 1);
}
