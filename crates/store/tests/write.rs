//! The write path against a real filesystem: `store::write::*`.
//!
//! Every condition here is **constructed**, not simulated: a directory where
//! the records belong, a file where a directory belongs, a directory with the
//! write bit cleared, a file truncated to a ragged length, a file truncated
//! under an open handle, a header whose counter outran its bytes, a header
//! region of zeros, and a second writer on the same month.
//!
//! Two conditions are **not** here and are not faked either — a full disk
//! (`ENOSPC`) and a read-only mount (`EROFS`). Neither can be produced without
//! privileged mounting, so the classifier that names them is driven directly
//! from `crates/store/src/file.rs`'s own test module with a real
//! `std::io::Error` of that kind. What is unverified is that the kernel hands
//! back that kind on this store's write; the mapping from it to a named
//! refusal is verified.
//!
//! The scratch root is a fresh directory per test under the host's temp
//! directory, named by process id and a counter, and removed on drop. No test
//! writes to a fixed path.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use store::crc::crc32c;
use store::file::{Action, Appended, BarFile, Conflict, StoreError};
use store::format::{
    Bar, FLAG_CHECKSUMS, FormatError, HEADER_LEN, OI_NULL, RECORD_LEN, RECORD_STRIDE,
};
use store::header::Header;
use store::layout::Layout;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Scratch
// ===========================================================================

/// Distinguishes two scratch roots taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary directory tree that removes itself.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!(
            "brutex-store-{}-{tag}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort: a test that made a directory unwritable restores it
        // itself, and a leaked scratch directory must never fail a test run.
        drop(fs::remove_dir_all(&self.root));
    }
}

// ===========================================================================
// Fixtures
// ===========================================================================

/// The open of the first one-minute bar of 2024-06-03, in microseconds.
const T0: i64 = 1_717_386_300_000_000;
/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;
/// The symbol id every test opens with unless it is testing the cross-check.
const SYMBOL: u32 = 26_000;

/// The `i`-th one-minute bar of the session, in paisa.
fn bar(index: i64) -> Bar {
    Bar {
        ts_micros: T0 + index * MINUTE,
        open: 2_345_600 + index,
        high: 2_345_900 + index,
        low: 2_345_100 + index,
        close: 2_345_700 + index,
        volume: 1_000 + index,
        open_interest: OI_NULL,
    }
}

/// `count` consecutive bars starting at index `from`.
fn batch(from: i64, count: i64) -> Vec<Bar> {
    (from..from + count).map(bar).collect()
}

fn parts(file: FileKind) -> PathParts<'static> {
    PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("2024-06"),
        file,
    }
}

fn bars_path() -> StorePath<'static> {
    StorePath::new(parts(FileKind::Bars)).expect("a legal path")
}

fn open(root: &Path) -> Result<BarFile, StoreError> {
    BarFile::open_or_create(root, bars_path(), SYMBOL)
}

/// The whole file, as bytes.
fn image(root: &Path) -> Vec<u8> {
    fs::read(bars_path().to_path_buf(root)).expect("the bar file")
}

/// A checksum of the whole file, for the idempotence assertions.
fn digest(root: &Path) -> u32 {
    crc32c(&image(root))
}

/// An open attempt, reduced to something comparable.
///
/// [`BarFile`] owns an open descriptor and a lock, so it cannot be `PartialEq`
/// and `assert_eq!` cannot take the whole `Result`. The record count is the
/// part a successful open is asserted on anyway.
fn outcome(result: Result<BarFile, StoreError>) -> Result<u64, StoreError> {
    result.map(|file| file.records())
}

// ===========================================================================
// Creating
// ===========================================================================

#[test]
fn a_fresh_month_lands_a_two_slot_header_region() {
    let scratch = Scratch::new("fresh");
    let file = open(scratch.root()).expect("create");

    assert_eq!(file.records(), 0, "a fresh month holds no records");
    assert_eq!(file.header().generation, 0);
    assert_eq!(file.header().symbol_id, SYMBOL);
    assert_eq!(file.header().timeframe_secs, 60);
    assert_eq!(file.layout(), Layout::V3, "born at version 3 since D-1571");
    assert_eq!(file.path(), bars_path().to_path_buf(scratch.root()));

    let bytes = image(scratch.root());
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        HEADER_LEN,
        "the whole header region is materialised, not left as a hole"
    );
    assert_eq!(
        &bytes[0..8],
        b"BRUTEXB3",
        "magic at byte 0: version 3 since D-1571"
    );
    assert!(
        bytes[64..16_384].iter().all(|b| *b == 0),
        "the rest of slot 0's span is reserved and zero"
    );
    assert!(
        bytes[16_384..].iter().all(|b| *b == 0),
        "slot 1 is empty until generation 1 is committed"
    );
}

#[test]
fn the_directory_tree_is_created_and_the_lock_is_the_bar_paths_sibling() {
    let scratch = Scratch::new("tree");
    let file = open(scratch.root()).expect("create");

    let bars = bars_path().to_path_buf(scratch.root());
    let lock = bars_path()
        .with_file(FileKind::Lock)
        .to_path_buf(scratch.root());
    assert!(bars.is_file(), "{} exists", bars.display());
    assert!(lock.is_file(), "{} exists", lock.display());
    assert_eq!(
        lock.parent(),
        bars.parent(),
        "the lock is derived from the same path, not concatenated"
    );
    assert_eq!(
        bars.strip_prefix(scratch.root()).unwrap(),
        Path::new("bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin"),
    );
    drop(file);
}

#[test]
fn the_lock_is_the_bar_paths_sibling() {
    // `StorePath::with_file` names this test.
    let bars = bars_path();
    assert_eq!(bars.file(), FileKind::Bars);
    let lock = bars.with_file(FileKind::Lock);
    assert_eq!(lock.file(), FileKind::Lock);
    assert_eq!(
        lock.to_string(),
        "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.lock"
    );
    assert_eq!(lock.with_file(FileKind::Bars), bars, "and back again");
    assert_eq!(lock.vendor(), Vendor::Groww);
    assert_eq!(lock.month(), YearMonth::new(2024, 6).unwrap());
    assert_eq!(lock.timeframe(), Timeframe::MINUTE_1);
}

#[test]
fn a_zero_byte_file_is_initialised_rather_than_condemned() {
    // What a crash between `create` and the first write leaves behind. It can
    // hold no record by definition, so initialising it loses nothing.
    let scratch = Scratch::new("zero");
    let bars = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(bars.parent().unwrap()).unwrap();
    fs::write(&bars, b"").unwrap();
    assert_eq!(fs::metadata(&bars).unwrap().len(), 0);

    let file = open(scratch.root()).expect("initialise");
    assert_eq!(file.records(), 0);
    assert_eq!(
        u64::try_from(image(scratch.root()).len()).unwrap(),
        HEADER_LEN
    );
}

/// **A SIBLING THAT HOLDS NO RECORDS IS REFUSED — AND THE OVERLAY DOES.**
///
/// This listed `Overlay` among the refused, and that was right while the
/// sidecar was a reserved name with nothing behind it. It has a geometry now —
/// 24-byte records, version 9 — and it is opened through this same door so it
/// inherits the header, the commit counter, the CRC and the block arithmetic
/// rather than growing a second copy of them.
///
/// `Checksums` and `Lock` stay refused, and that distinction is the whole
/// point: they are not record files at all. Opening one here would read its
/// first bytes as a header and then serve whatever followed as records.
#[test]
fn a_sibling_that_holds_no_records_is_refused_and_the_overlay_is_not_one() {
    let scratch = Scratch::new("sibling");
    for kind in [
        FileKind::Checksums,
        FileKind::Lock,
        FileKind::OverlayChecksums,
        FileKind::GreekChecksums,
    ] {
        let path = StorePath::new(parts(kind)).expect("a legal path");
        assert_eq!(
            outcome(BarFile::open_or_create(scratch.root(), path, SYMBOL)),
            Err(StoreError::NotABarPath { found: kind }),
            "{kind:?} holds no records and must not be opened as though it did"
        );
        assert_eq!(
            outcome(BarFile::open_existing(scratch.root(), path, SYMBOL)),
            Err(StoreError::NotABarPath { found: kind }),
        );
    }

    // THE OVERLAY OPENS, and at its OWN geometry. Reading it at the bar's
    // stride is the dangerous direction and it would not announce itself: the
    // header region is the same shape in both, so the header validates, the
    // CRC passes, and every field afterwards comes from the wrong offset.
    let path = StorePath::new(parts(FileKind::Overlay)).expect("a legal path");
    let file = BarFile::open_or_create(scratch.root(), path, SYMBOL)
        .expect("the overlay is a record file and opens as one");
    assert_eq!(file.records(), 0, "a fresh overlay holds nothing yet");
}

// ===========================================================================
// Appending and reading back
// ===========================================================================

#[test]
fn the_first_bar_lands_at_the_computed_offset() {
    let scratch = Scratch::new("first");
    let mut file = open(scratch.root()).expect("create");

    let one = bar(0);
    assert_eq!(
        file.append(&[one]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 1
        })
    );

    let bytes = image(scratch.root());
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        HEADER_LEN + RECORD_STRIDE
    );
    let at = usize::try_from(Layout::V2.offset_of(0).unwrap()).unwrap();
    assert_eq!(
        &bytes[at..at + RECORD_LEN],
        &one.image()[..],
        "the bytes on disk are Bar::image, not a second encoder"
    );
    assert_eq!(file.read_record(0), Ok(one));
    assert_eq!(file.header().first_ts_micros, one.ts_micros);
    assert_eq!(file.header().last_ts_micros, one.ts_micros);
}

#[test]
fn a_record_reads_back_at_its_computed_offset() {
    // One multiply, one add, one 56-byte read: the address is the index, at
    // both ends of the file and in the middle.
    let scratch = Scratch::new("index");
    let mut file = open(scratch.root()).expect("create");
    let day = batch(0, 375);
    assert_eq!(
        file.append(&day),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 375
        })
    );

    let bytes = image(scratch.root());
    for index in [0u64, 1, 72, 73, 145, 200, 374] {
        let want = day[usize::try_from(index).unwrap()];
        assert_eq!(file.read_record(index), Ok(want), "record {index}");
        let at = usize::try_from(Layout::V2.offset_of(index).unwrap()).unwrap();
        assert_eq!(
            &bytes[at..at + RECORD_LEN],
            &want.image()[..],
            "record {index} sits at header_len + index * stride"
        );
    }
    assert_eq!(
        file.read_record(375),
        Err(StoreError::NotCommitted {
            index: 375,
            n_valid: 375
        })
    );
}

#[test]
fn the_committed_length_is_exactly_durable_through() {
    let scratch = Scratch::new("durable");
    let mut file = open(scratch.root()).expect("create");
    let mut written = 0i64;
    for count in [1i64, 10, 73, 100] {
        assert!(matches!(
            file.append(&batch(written, count)),
            Ok(Appended::Committed { .. })
        ));
        written += count;
        let commit = file.header().commit().expect("a committable header");
        assert_eq!(
            u64::try_from(image(scratch.root()).len()).unwrap(),
            commit.durable_through,
            "every byte the commit publishes is a byte the file has"
        );
    }
    assert_eq!(file.records(), 184);
}

#[test]
fn consecutive_commits_alternate_between_the_two_slots() {
    let scratch = Scratch::new("slots");
    let mut file = open(scratch.root()).expect("create");
    assert!(file.append(&batch(0, 2)).is_ok());
    let after_one = image(scratch.root());
    assert!(file.append(&batch(2, 2)).is_ok());
    let after_two = image(scratch.root());

    // Generation 1 went to slot 1; generation 2 went back to slot 0. The slot
    // holding the previous commit is never the slot being written, which is
    // the whole of the header's crash argument.
    assert_ne!(
        &after_one[0..64],
        &after_two[0..64],
        "generation 2 rewrote slot 0"
    );
    assert_eq!(
        &after_one[16_384..16_448],
        &after_two[16_384..16_448],
        "and left generation 1 in slot 1 untouched"
    );
    assert_eq!(file.header().generation, 2);
    assert_eq!(file.records(), 4);
}

#[test]
fn reopening_a_month_sees_every_committed_record() {
    let scratch = Scratch::new("reopen");
    let day = batch(0, 50);
    {
        let mut file = open(scratch.root()).expect("create");
        assert!(file.append(&day).is_ok());
    }
    let file = open(scratch.root()).expect("reopen");
    assert_eq!(file.records(), 50);
    assert_eq!(file.header().generation, 1);
    assert_eq!(file.read_record(0), Ok(day[0]));
    assert_eq!(file.read_record(49), Ok(day[49]));
}

// ===========================================================================
// Idempotence
// ===========================================================================

#[test]
fn re_appending_the_same_batch_leaves_the_file_byte_identical() {
    let scratch = Scratch::new("idem");
    let mut file = open(scratch.root()).expect("create");
    let day = batch(0, 40);
    assert_eq!(
        file.append(&day),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 40
        })
    );

    let before = image(scratch.root());
    assert_eq!(
        file.append(&day),
        Ok(Appended::AlreadyPresent {
            first_index: 0,
            n_valid: 40
        }),
        "a re-pull of the same window is a no-op, not a duplicate"
    );
    let after = image(scratch.root());
    assert_eq!(
        digest(scratch.root()),
        crc32c(&before),
        "the whole file checksums the same either side"
    );
    assert_eq!(before, after, "and is equal byte for byte");

    // The tail alone, re-offered, is also already present.
    assert_eq!(
        file.append(&day[30..]),
        Ok(Appended::AlreadyPresent {
            first_index: 30,
            n_valid: 40
        })
    );
    assert_eq!(image(scratch.root()), before);
}

#[test]
fn an_overlapping_batch_with_different_bars_is_refused() {
    let scratch = Scratch::new("overlap");
    let mut file = open(scratch.root()).expect("create");
    assert!(file.append(&batch(0, 10)).is_ok());
    let before = image(scratch.root());

    let mut altered = batch(5, 10);
    altered[0].close += 1;
    assert_eq!(
        file.append(&altered),
        Err(StoreError::OverlapDisagrees {
            path: file.path().to_path_buf(),
            at: 0,
            ts_micros: T0 + 5 * MINUTE,
            conflict: Conflict::Restated,
        }),
        "a re-pull landing on top of committed bars with different values"
    );
    assert_eq!(image(scratch.root()), before, "and nothing was written");

    // A LONGER BATCH WHOSE OVERLAP MATCHES IS A RESUME, AND IT LANDS.
    //
    // This asserted `is_err()` — "a batch longer than the file cannot be its
    // tail either" — which is true and was the wrong conclusion. The file holds
    // 0..10 and the batch is 0..20: the first ten are byte-identical to what is
    // stored and the last ten follow. Refusing it is refusing the normal shape
    // of a resumed backfill.
    //
    // Measured on a real Groww pull before this changed: 1,260 rows read, 0
    // bars stored, because the window overlapped a month that already held its
    // first day. `CLAUDE.md` §3 rule 5 promises reruns are safe; they were not.
    assert!(
        file.append(&batch(0, 20)).is_ok(),
        "an overlap that matches byte for byte is a resume, not a conflict"
    );
    assert_eq!(
        file.records(),
        20,
        "and only the ten that follow were appended — not twenty, not zero"
    );

    // THE CONFLICT ABOVE STILL REFUSES. A differing bar in the overlap is a
    // vendor restating history, and that is not something to swallow: the
    // `altered` batch earlier in this test is still an error, and this asserts
    // the two cases stayed apart rather than one swallowing the other.
    let mut altered_long = batch(0, 30);
    altered_long[3].close += 1;
    assert!(
        file.append(&altered_long).is_err(),
        "a differing bar inside the overlap is refused, however much valid \
         suffix follows it — silently keeping the suffix would hide a vendor \
         restating history"
    );
    assert_eq!(file.records(), 20, "and nothing was written");
}

// ===========================================================================
// Refusals before a byte is written
// ===========================================================================

#[test]
fn an_empty_append_is_refused() {
    let scratch = Scratch::new("empty");
    let mut file = open(scratch.root()).expect("create");
    let before = image(scratch.root());
    assert_eq!(file.append::<Bar>(&[]), Err(StoreError::EmptyBatch));
    assert_eq!(image(scratch.root()), before);
    assert_eq!(file.header().generation, 0, "no generation was spent");
}

#[test]
fn an_out_of_order_batch_never_reaches_the_disk() {
    let scratch = Scratch::new("order");
    let mut file = open(scratch.root()).expect("create");
    let before = image(scratch.root());

    let mut day = batch(0, 5);
    day.swap(2, 3);
    assert_eq!(
        file.append(&day),
        Err(StoreError::BatchNotOrdered {
            at: 3,
            previous: T0 + 3 * MINUTE,
            next: T0 + 2 * MINUTE,
        })
    );

    let mut repeated = batch(0, 3);
    repeated[2] = repeated[1];
    assert_eq!(
        file.append(&repeated),
        Err(StoreError::BatchNotOrdered {
            at: 2,
            previous: T0 + MINUTE,
            next: T0 + MINUTE,
        }),
        "equal timestamps are not increasing either"
    );
    assert_eq!(image(scratch.root()), before);
}

#[test]
fn an_impossible_bar_never_reaches_the_disk() {
    let scratch = Scratch::new("insane");
    let mut file = open(scratch.root()).expect("create");
    let before = image(scratch.root());

    let mut day = batch(0, 4);
    day[2].high = day[2].low - 1;
    assert_eq!(file.append(&day), Err(StoreError::ImpossibleBar { at: 2 }));
    assert_eq!(image(scratch.root()), before);
    assert_eq!(file.records(), 0);
}

#[test]
fn a_record_past_the_counter_is_refused() {
    let scratch = Scratch::new("past");
    let mut file = open(scratch.root()).expect("create");
    assert_eq!(
        file.read_record(0),
        Err(StoreError::NotCommitted {
            index: 0,
            n_valid: 0
        })
    );
    assert!(file.append(&batch(0, 3)).is_ok());
    assert!(file.read_record(2).is_ok());
    assert_eq!(
        file.read_record(3),
        Err(StoreError::NotCommitted {
            index: 3,
            n_valid: 3
        })
    );
}

// ===========================================================================
// Two writers
// ===========================================================================

#[test]
fn a_second_writer_is_refused_while_the_month_is_held() {
    let scratch = Scratch::new("lock");
    let first = open(scratch.root()).expect("create");
    let lock_path = bars_path()
        .with_file(FileKind::Lock)
        .to_path_buf(scratch.root());

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Locked {
            path: lock_path.clone()
        }),
        "the commit's crash argument assumes exactly one writer"
    );
    drop(first);

    let second = open(scratch.root()).expect("the month is free again");
    assert_eq!(second.records(), 0);
    assert!(lock_path.is_file(), "the lock file itself is not deleted");
}

// ===========================================================================
// What the host refuses
// ===========================================================================

#[test]
fn a_directory_where_the_records_belong_is_named() {
    let scratch = Scratch::new("isdir");
    let bars = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(&bars).unwrap();
    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::IsADirectory {
            path: bars,
            action: Action::Open
        })
    );
}

#[test]
fn a_lock_path_that_is_a_directory_is_named() {
    // The lock is opened before the records are, so this is the refusal an
    // operator sees when the month cannot be claimed at all.
    let scratch = Scratch::new("lockdir");
    let lock = bars_path()
        .with_file(FileKind::Lock)
        .to_path_buf(scratch.root());
    fs::create_dir_all(&lock).unwrap();
    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::IsADirectory {
            path: lock,
            action: Action::Open
        })
    );
}

#[test]
fn a_file_where_a_directory_belongs_is_named() {
    let scratch = Scratch::new("notdir");
    let root = scratch.root().join("plain");
    fs::write(&root, b"not a directory").unwrap();
    // A ROOT that is a file is refused naming the root, before anything below
    // it is attempted (D-1522).
    assert_eq!(
        outcome(BarFile::open_or_create(&root, bars_path(), SYMBOL)),
        Err(StoreError::NotADirectory {
            path: root.clone(),
            action: Action::Open,
        })
    );
    // A file where a directory BELOW the root belongs is refused naming the
    // directory that could not be created.
    let root = scratch.root().join("mounted");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("bars"), b"not a directory").unwrap();
    let refusal = outcome(BarFile::open_or_create(&root, bars_path(), SYMBOL));
    assert_eq!(
        refusal,
        Err(StoreError::NotADirectory {
            path: bars_path()
                .to_path_buf(&root)
                .parent()
                .unwrap()
                .to_path_buf(),
            action: Action::CreateDir,
        })
    );
}

#[test]
fn a_directory_that_refuses_a_write_is_named() {
    support::where_permission_binds(
        "a_directory_that_refuses_a_write_is_named",
        a_directory_that_refuses_a_write_is_named_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
fn a_directory_that_refuses_a_write_is_named_body() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new("denied");
    let root = scratch.root().join("readonly");
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, PermissionsExt::from_mode(0o555)).unwrap();

    let refusal = outcome(BarFile::open_or_create(&root, bars_path(), SYMBOL));

    // Restore before the assertion so a failure still cleans up.
    fs::set_permissions(&root, PermissionsExt::from_mode(0o755)).unwrap();
    assert_eq!(
        refusal,
        Err(StoreError::Denied {
            path: bars_path()
                .to_path_buf(&root)
                .parent()
                .unwrap()
                .to_path_buf(),
            action: Action::CreateDir,
        }),
        "running this suite as root would defeat the condition, not the test"
    );
}

// ===========================================================================
// What the bytes refuse
// ===========================================================================

#[test]
fn a_ragged_tail_is_refused_by_length() {
    let scratch = Scratch::new("ragged");
    {
        let mut file = open(scratch.root()).expect("create");
        assert!(file.append(&batch(0, 4)).is_ok());
    }
    let bars = bars_path().to_path_buf(scratch.root());
    let len = HEADER_LEN + 3 * RECORD_STRIDE + 17;
    fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .unwrap()
        .set_len(len)
        .unwrap();

    // The counter says four records and the bytes stop 39 short of the fourth,
    // so `Header::validate` rejects generation 1 and the header search walks
    // back to generation 0, whose counter is zero. That is what has to be
    // refused: the walk-back is a recovery for a header that outran its
    // records, and applying it here would reclassify three records a commit
    // published as scratch the next append overwrites.
    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Format {
            path: bars,
            source: FormatError::CounterExceedsFile,
        }),
        "`CounterExceedsFile`, not `RaggedTail`, and the variant changed on \
         purpose: what is wrong with this file is the 39 bytes MISSING below \
         the commit, not the 17 that happen to be extra above the last whole \
         record. Cut the same file to a record boundary and `RaggedTail`'s own \
         `extra` would read zero while the same three records are just as \
         lost, so the length-shaped variant could not even state the condition"
    );
}

#[test]
fn a_counter_behind_its_bytes_opens_and_a_counter_ahead_of_them_is_refused() {
    // THE TWO FILES BELOW ARE THE SAME LENGTH AND THE SAME SHAPE, AND THEY ARE
    // OPPOSITE CONDITIONS. Both end up 32,953 bytes: the 32,768-byte header
    // region, three whole records, and seventeen bytes that are not a record.
    // Every function of the length agrees about them — `ragged_tail_bytes` is
    // 17 for both, `capacity_for` is 3 for both — so a check written against
    // the length has to collapse one into the other, and this file's history
    // is that mistake made in both directions in turn: first every ragged file
    // was refused, which bricked a month for an interrupted append; then every
    // ragged file was accepted, which silently un-committed three bars.
    //
    //   * BEHIND — three records committed, then seventeen bytes of a fourth
    //     that no header slot ever published. The commit counter is behind the
    //     bytes. An append died before its commit; the next append writes at
    //     `offset_of(3)` and covers them. Accept, and log the count.
    //   * AHEAD — four records committed, then the file cut 39 bytes short of
    //     the fourth. The commit counter is ahead of the bytes. Truncation or
    //     corruption, and `CLAUDE.md` §4 forbids the fallback that would make
    //     it look like the first case. Refuse.
    //
    // Asserted side by side, in one test, so a future edit cannot satisfy one
    // of them and discover the other only in production.
    let torn = HEADER_LEN + 3 * RECORD_STRIDE + 17;

    let behind = Scratch::new("counter-behind");
    {
        let mut file = open(behind.root()).expect("create");
        assert!(file.append(&batch(0, 3)).is_ok());
    }
    let behind_bars = bars_path().to_path_buf(behind.root());
    let mut bytes = fs::read(&behind_bars).unwrap();
    bytes.extend_from_slice(&[9u8; 17]);
    fs::write(&behind_bars, &bytes).unwrap();
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        torn,
        "the premise: this is the length the other file will also have"
    );

    let opened = open(behind.root()).expect("a month is not lost to bytes no commit claims");
    assert_eq!(opened.records(), 3, "the counter is untouched");
    assert_eq!(opened.read_record(2), Ok(bar(2)), "and so are the bars");
    drop(opened);

    let ahead = Scratch::new("counter-ahead");
    {
        let mut file = open(ahead.root()).expect("create");
        assert!(file.append(&batch(0, 4)).is_ok());
    }
    let ahead_bars = bars_path().to_path_buf(ahead.root());
    fs::OpenOptions::new()
        .write(true)
        .open(&ahead_bars)
        .unwrap()
        .set_len(torn)
        .unwrap();

    assert_eq!(
        outcome(open(ahead.root())),
        Err(StoreError::Format {
            path: ahead_bars.clone(),
            source: FormatError::CounterExceedsFile,
        }),
        "same length and same raggedness as the month above, and the opposite \
         answer: a slot still claims four records, so the three whole ones on \
         disk were published and are not scratch"
    );

    // AND THE REFUSAL IS NOT ABOUT THE SEVENTEEN BYTES. Cut the same file to a
    // record boundary: the remainder is zero, which is the only quantity the
    // refusal this replaced could measure, and the three committed records are
    // exactly as gone. The old length-shaped check opened this one without a
    // word — it is the case that variant was structurally unable to state.
    let aligned = HEADER_LEN + 3 * RECORD_STRIDE;
    assert_eq!(
        Layout::V2.ragged_tail_bytes(aligned),
        0,
        "the premise: nothing about this length is ragged"
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&ahead_bars)
        .unwrap()
        .set_len(aligned)
        .unwrap();
    assert_eq!(
        outcome(open(ahead.root())),
        Err(StoreError::Format {
            path: ahead_bars,
            source: FormatError::CounterExceedsFile,
        }),
        "a truncation that lands on a record boundary is still a truncation"
    );
}

/// A truncation back to the bare header is REFUSED, not quietly fallen back from.
///
/// # This test used to assert the opposite, and it was wrong in the direction
/// that loses data
///
/// It was called `a_header_that_outran_its_file_falls_back_one_generation` and
/// it asserted that ten committed bars could vanish while the file opened
/// clean at `records() == 0`. Its premise — *"the records never reached the
/// disk; the header slot did"* — is a state [`BarFile::append`] cannot produce:
/// it writes the records and `sync_all`s them BEFORE writing any slot that
/// names them, and a slot torn mid-write fails its CRC and does not decode at
/// all. `set_len(HEADER_LEN)` does not simulate that ordering. It destroys ten
/// committed records.
///
/// The test directly above this one already asserts `CounterExceedsFile` for
/// *"a truncation that lands on a record boundary"*, and `HEADER_LEN` is record
/// boundary zero — so the two tests asserted opposite answers to one question
/// and the more dangerous one won on a technicality of where an `if` sat.
///
/// The refusal it now expects comes from hoisting that `if` out of the
/// `discarded != 0` arm in `BarFile::validated`. A truncation landing exactly
/// on an older commit's extent leaves `discarded == 0` by construction, which
/// is why the guard could never see it; `claimed` — the highest `n_valid` over
/// every slot that still decodes — proved the loss the whole time, from a local
/// one line above the guard that ignored it.
#[test]
fn a_truncation_back_to_the_header_is_refused_rather_than_silently_accepted() {
    let scratch = Scratch::new("outran");
    {
        let mut file = open(scratch.root()).expect("create");
        assert!(file.append(&batch(0, 10)).is_ok());
    }
    let bars = bars_path().to_path_buf(scratch.root());
    fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .unwrap()
        .set_len(HEADER_LEN)
        .unwrap();

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Format {
            path: bars,
            source: FormatError::CounterExceedsFile,
        }),
        "ten committed bars are gone and a surviving slot still claims them — \
         opening clean here is how a month reports complete while holding nothing"
    );
}

#[test]
fn a_header_that_outran_every_generation_is_refused() {
    let scratch = Scratch::new("outran-all");
    {
        let mut file = open(scratch.root()).expect("create");
        for round in 0..3i64 {
            assert!(file.append(&batch(round * 10, 10)).is_ok());
        }
        assert_eq!(file.records(), 30);
    }
    // Both surviving slots — generations 2 and 3 — claim more records than
    // five records' worth of bytes can hold.
    let bars = bars_path().to_path_buf(scratch.root());
    fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .unwrap()
        .set_len(HEADER_LEN + 5 * RECORD_STRIDE)
        .unwrap();

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Format {
            path: bars,
            source: FormatError::CounterExceedsFile
        })
    );
}

#[test]
fn a_destroyed_header_region_is_refused() {
    let scratch = Scratch::new("destroyed");
    {
        let mut file = open(scratch.root()).expect("create");
        assert!(file.append(&batch(0, 3)).is_ok());
    }
    let bars = bars_path().to_path_buf(scratch.root());
    let mut bytes = fs::read(&bars).unwrap();
    for byte in bytes.iter_mut().take(usize::try_from(HEADER_LEN).unwrap()) {
        *byte = 0;
    }
    fs::write(&bars, &bytes).unwrap();

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Format {
            path: bars,
            source: FormatError::NoValidHeader
        }),
        "every copy of the header is gone; anything else would be a guess"
    );
}

#[test]
fn a_file_shorter_than_its_header_region_is_refused() {
    let scratch = Scratch::new("stub");
    let bars = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(bars.parent().unwrap()).unwrap();

    // A genuine, checksum-valid slot 0 in a file with no room for slot 1.
    // Returning the commit that happened to fit would silently lose every
    // record committed since, so it is refused instead.
    let commit = Header::genesis(SYMBOL, 60, FLAG_CHECKSUMS)
        .commit()
        .expect("genesis");
    let mut stub = vec![0u8; 100];
    stub[..commit.bytes.len()].copy_from_slice(&commit.bytes);
    fs::write(&bars, &stub).unwrap();

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Format {
            path: bars,
            source: FormatError::HeaderRegionTooShort { slots: 0, need: 2 }
        })
    );
}

#[test]
fn a_truncation_under_an_open_handle_is_a_short_read() {
    let scratch = Scratch::new("shortread");
    let mut file = open(scratch.root()).expect("create");
    assert!(file.append(&batch(0, 10)).is_ok());

    // A whole number of records, so nothing about the length is ragged — the
    // header simply names records the bytes no longer have.
    let bars = bars_path().to_path_buf(scratch.root());
    fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .unwrap()
        .set_len(HEADER_LEN + 5 * RECORD_STRIDE)
        .unwrap();

    // RECORD 4 IS PHYSICALLY THERE AND IS STILL REFUSED, and that changed when
    // reads began verifying the block checksum.
    //
    // This used to assert `Ok(bar(4))` — "the records still there". They are.
    // But a checksum covers a BLOCK, and this block's domain is the ten records
    // the header commits, of which five are gone. So the seal cannot be
    // recomputed, and serving record 4 would be serving a record whose
    // integrity nobody can check while the file's own header says it was
    // sealed. That is the fallback §4 bans, and it is the reason the checksum
    // is read at all.
    //
    // It refuses at the block's extent, not at the record's: `offset` is where
    // the covered range runs past the file, and `asked == read == 280` is the
    // five records that survive against the ten the seal needs.
    let truncated = HEADER_LEN + 5 * RECORD_STRIDE;
    assert_eq!(
        file.read_record(4),
        Err(StoreError::ShortRead {
            path: bars.clone(),
            offset: truncated,
            asked: 5 * RECORD_LEN,
            read: 5 * RECORD_LEN,
        }),
        "a block that cannot be verified is not served, even where its bytes survive"
    );
    // AND A RECORD THAT IS GONE REFUSES EARLIER, at its own offset rather than
    // the block's. Since D-1433 a read fetches the whole block in one `pread`
    // and, when that comes up short, re-reads the record alone to name it —
    // so record 9's own missing bytes are what is reported, and the block seal
    // is never reached. Two different refusals for two different
    // facts: "this record is not there" and "this block cannot be checked".
    assert_eq!(
        file.read_record(9),
        Err(StoreError::ShortRead {
            path: bars,
            offset: HEADER_LEN + 9 * RECORD_STRIDE,
            asked: RECORD_LEN,
            read: 0,
        })
    );
}

#[test]
fn a_re_pull_against_a_truncated_file_cannot_claim_the_bars_are_already_there() {
    // The duplicate check reads the records it is comparing against. When
    // those bytes are gone, "already present" would be a claim about bytes
    // nobody can see, so the read's refusal comes back out instead.
    let scratch = Scratch::new("repull-short");
    let mut file = open(scratch.root()).expect("create");
    assert!(file.append(&batch(0, 10)).is_ok());

    let bars = bars_path().to_path_buf(scratch.root());
    fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .unwrap()
        .set_len(HEADER_LEN + 5 * RECORD_STRIDE)
        .unwrap();

    assert_eq!(
        file.append(&batch(5, 5)),
        Err(StoreError::ShortRead {
            path: bars,
            offset: HEADER_LEN + 5 * RECORD_STRIDE,
            asked: RECORD_LEN,
            read: 0,
        })
    );
}

#[test]
fn a_file_written_for_another_symbol_is_refused() {
    let scratch = Scratch::new("symbol");
    drop(open(scratch.root()).expect("create"));
    assert_eq!(
        outcome(BarFile::open_or_create(
            scratch.root(),
            bars_path(),
            SYMBOL + 1
        )),
        Err(StoreError::SymbolMismatch {
            path: bars_path().to_path_buf(scratch.root()),
            stored: SYMBOL,
            asked: SYMBOL + 1,
        }),
        "symbol_id is a cross-check, and this is the check"
    );
}

#[test]
fn a_file_written_at_another_timeframe_is_refused() {
    let scratch = Scratch::new("timeframe");
    let bars = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(bars.parent().unwrap()).unwrap();

    // A well-formed header region for five-minute bars, written through the
    // same public commit the writer uses.
    let commit = Header::genesis(SYMBOL, 300, FLAG_CHECKSUMS)
        .commit()
        .expect("genesis");
    let mut region = vec![0u8; usize::try_from(HEADER_LEN).unwrap()];
    let at = usize::try_from(commit.offset).unwrap();
    region[at..at + commit.bytes.len()].copy_from_slice(&commit.bytes);
    fs::write(&bars, &region).unwrap();

    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::TimeframeMismatch {
            path: bars,
            stored: 300,
            asked: 60,
        })
    );
}

// ===========================================================================
// Every refusal says what it refused
// ===========================================================================

#[test]
fn every_action_has_a_word_of_its_own() {
    let actions = [
        Action::CreateDir,
        Action::Open,
        Action::Lock,
        Action::Measure,
        Action::Read,
        Action::Write,
        Action::Sync,
    ];
    let mut seen: Vec<&str> = actions.iter().map(|a| a.as_str()).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), actions.len(), "no two actions share a word");
    for action in actions {
        assert_eq!(action.to_string(), action.as_str());
        assert!(!action.as_str().is_empty());
    }
}

/// The file every rendering case names.
fn rendered_path() -> PathBuf {
    PathBuf::from("/store/bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin")
}

/// Renders each refusal and checks it says the thing that sends an operator to
/// the right place.
fn each_renders(cases: Vec<(StoreError, &str)>) {
    assert!(!cases.is_empty(), "a table of no cases proves nothing");
    for (refusal, needle) in cases {
        let rendered = refusal.to_string();
        assert!(
            rendered.contains(needle),
            "{refusal:?} rendered as {rendered:?}, wanted {needle:?}"
        );
        assert!(
            std::error::Error::source(&refusal).is_none(),
            "the host's error is reduced to its kind and errno, not chained"
        );
    }
}

#[test]
fn every_host_refusal_names_the_file_and_the_operation() {
    let path = rendered_path();
    each_renders(vec![
        (
            StoreError::NotABarPath {
                found: FileKind::Overlay,
            },
            ".ovl",
        ),
        (StoreError::Locked { path: path.clone() }, "another writer"),
        (
            StoreError::DiskFull {
                path: path.clone(),
                action: Action::Write,
            },
            "disk full writing",
        ),
        (
            StoreError::Denied {
                path: path.clone(),
                action: Action::Open,
            },
            "permission denied opening",
        ),
        (
            StoreError::ReadOnly {
                path: path.clone(),
                action: Action::Write,
            },
            "read-only filesystem writing",
        ),
        (
            StoreError::IsADirectory {
                path: path.clone(),
                action: Action::Open,
            },
            "is a directory, opening it",
        ),
        (
            StoreError::NotADirectory {
                path: path.clone(),
                action: Action::CreateDir,
            },
            "is not a directory",
        ),
        (
            StoreError::Missing {
                path: path.clone(),
                action: Action::Measure,
            },
            "does not exist, measuring it",
        ),
        (
            StoreError::Io {
                path,
                action: Action::Sync,
                kind: std::io::ErrorKind::Other,
                code: Some(5),
            },
            "errno Some(5)",
        ),
    ]);
}

#[test]
fn every_content_refusal_names_the_numbers_it_refused() {
    let path = rendered_path();
    each_renders(vec![
        (
            StoreError::ShortWrite {
                path: path.clone(),
                offset: 32_768,
                asked: 56,
                wrote: 0,
            },
            "stalled at offset 32768",
        ),
        (
            StoreError::ShortRead {
                path: path.clone(),
                offset: 32_768,
                asked: 56,
                read: 0,
            },
            "ended at offset 32768",
        ),
        (
            StoreError::RaggedTail {
                path: path.clone(),
                len: 32_785,
                extra: 17,
            },
            "17 past the last whole record",
        ),
        (
            StoreError::Format {
                path: path.clone(),
                source: FormatError::CounterExceedsFile,
            },
            "n_valid claims more records",
        ),
        (
            StoreError::SymbolMismatch {
                path: path.clone(),
                stored: 1,
                asked: 2,
            },
            "holds symbol 1, not 2",
        ),
        (
            StoreError::TimeframeMismatch {
                path,
                stored: 300,
                asked: 60,
            },
            "holds 300-second bars, not 60",
        ),
        (
            StoreError::NotCommitted {
                index: 9,
                n_valid: 5,
            },
            "record 9 is past the 5 committed",
        ),
        (StoreError::EmptyBatch, "no records in it"),
        (
            StoreError::BatchNotOrdered {
                at: 3,
                previous: 10,
                next: 9,
            },
            "9 does not follow 10",
        ),
        (StoreError::ImpossibleBar { at: 2 }, "impossible OHLC"),
    ]);
}

// ===========================================================================
// The two doors
// ===========================================================================

/// **BOTH DOORS AGREE, AND NOW BY CONSTRUCTION RATHER THAN BY COINCIDENCE.**
///
/// `BarFile::open_existing`'s doc has always claimed of itself and
/// `open_or_create`: "both call `Self::validated` so they cannot drift into
/// disagreeing about what a well-formed month is." **That sentence was false.**
/// `validated` had exactly one caller, and `open_or_create` carried its own
/// copy of the four checks — twenty-eight lines that happened to be byte for
/// byte identical.
///
/// Nothing was wrong with the values, which is exactly why it was worth fixing
/// and why this test exists. The comment asserted a guarantee that no mechanism
/// enforced, so the two agreed only until somebody edited one of them. The
/// duplicate is gone; this pins the property the sentence promises.
///
/// It also covers a door that had **no test at all**. A coverage measurement on
/// 2026-08-10 found `open_existing` and `validated` dark at 40 of 40 lines,
/// while `/bars` — a routed, shipping, user-facing read path — sits on top of
/// them.
#[test]
fn the_reader_door_opens_what_the_writer_wrote_and_refuses_exactly_what_it_refuses() {
    let scratch = Scratch::new("both-doors");
    let root = scratch.root();

    // ABSENT IS `Missing`, AND THE READER CREATES NOTHING LOOKING.
    // The module header promises "nothing, ever — it creates no directory, no
    // bar file and no lock". A reader that conjured a lock file into being
    // would be performing the very write this door exists to avoid.
    let absent = BarFile::open_existing(root, bars_path(), SYMBOL);
    assert!(
        matches!(absent, Err(StoreError::Missing { .. })),
        "an absent month is Missing, not an empty file: {absent:?}"
    );
    let lock_path = StorePath::new(parts(FileKind::Lock))
        .expect("a legal path")
        .to_path_buf(root);
    assert!(
        !bars_path().to_path_buf(root).exists(),
        "the reader created a bar file"
    );
    assert!(!lock_path.exists(), "the reader created a lock file");

    // Write a month through the writer door, then release its exclusive lock.
    {
        let mut writer = open(root).expect("the writer creates the month");
        assert_eq!(
            writer.append(&batch(0, 3)).expect("appends"),
            Appended::Committed {
                first_index: 0,
                n_valid: 3,
            }
        );
    }

    // The reader door opens what the writer wrote and sees the same records.
    let reader = BarFile::open_existing(root, bars_path(), SYMBOL).expect("opens what was written");
    assert_eq!(reader.records(), 3, "the reader sees the committed records");
    assert_eq!(reader.header().symbol_id, SYMBOL);
    assert_eq!(
        reader.layout(),
        Layout::V3,
        "born at version 3 since D-1571"
    );
    drop(reader);

    // ─── AND THE READER CREATES NO SIDECAR, WHICH IS THE HALF THAT WAS FALSE
    //
    // "creates no directory, no bar file and no lock" held for all three, and
    // the CHECKSUM SIDECAR was opened `create(true)` two lines below the check
    // — so the promise covered the artefacts anyone looked at and not the one
    // beside them.
    //
    // It was invisible while `FLAG_CHECKSUMS` was clear in every file this
    // module wrote, which the module header still claimed. `initialise` sets
    // the flag, so the branch is taken on EVERY open and the read door created
    // on every miss. Two live consequences: a GET against a month whose `.crc`
    // is absent left a ZERO-BYTE sidecar, and a zero sum read back against a
    // real block reports a healthy month as CORRUPT; and a store on a read-only
    // mount refused the read outright, because it tried to write.
    let sidecar = bars_path().with_file(FileKind::Checksums).to_path_buf(root);
    // IT MUST HAVE EXISTED FIRST, or the path below is wrong and every
    // assertion after it passes for free — a file that was never there cannot
    // be created again. This is the vacuity the negative assertion invites.
    assert!(
        sidecar.exists(),
        "the writer's append seals a block, so the sidecar is on disk here: {}",
        sidecar.display()
    );
    std::fs::remove_file(&sidecar).expect("take the sidecar away");
    assert!(!sidecar.exists(), "and it is gone before the read");
    let after = BarFile::open_existing(root, bars_path(), SYMBOL).expect("still opens");
    assert_eq!(
        after.records(),
        3,
        "the month still reads without its sidecar — an absent checksum file is \
         a fact to report, not a reason to refuse the bars"
    );
    drop(after);
    assert!(
        !sidecar.exists(),
        "THE READ DOOR CREATED THE SIDECAR. A GET that makes a zero-byte \
         checksum file turns the next verification into a false CORRUPT, and \
         §3 rule 8 forbids rewriting the month to clear it: {}",
        sidecar.display()
    );

    // THE SHARED VALIDATION, WHICH IS THE WHOLE POINT. A month whose stored
    // symbol is not the one asked for must be refused by BOTH doors, with the
    // SAME error — that is the sentence, expressed as an assertion. Compared as
    // values rather than by shape, so a door that refused for a different
    // reason, or named a different id, fails here.
    let other = SYMBOL + 1;
    let by_writer = outcome(BarFile::open_or_create(root, bars_path(), other));
    let by_reader = outcome(BarFile::open_existing(root, bars_path(), other));
    assert!(
        matches!(by_writer, Err(StoreError::SymbolMismatch { .. })),
        "the writer door must refuse a symbol mismatch: {by_writer:?}"
    );
    assert_eq!(
        by_writer, by_reader,
        "the two doors disagreed about the same month — which is precisely the \
         drift `validated` exists to make impossible"
    );

    // And the same for the timeframe, so the agreement is not one lucky arm.
    let other_tf = PathParts {
        timeframe: Timeframe::DAY_1,
        ..parts(FileKind::Bars)
    };
    let other_tf = StorePath::new(other_tf).expect("a legal path");
    let w = outcome(BarFile::open_or_create(root, other_tf, SYMBOL));
    let r = outcome(BarFile::open_existing(root, other_tf, SYMBOL));
    assert_eq!(
        w, r,
        "a different timeframe is a different month to one door and not the other"
    );
}

// ===========================================================================
// The overlay, written and read back
// ===========================================================================

/// **AN OVERLAY ROUND-TRIPS THROUGH THE SAME WRITER THE BARS USE.**
///
/// This is the assertion the whole generalisation was for. A second writer for
/// the sidecar would have copied a hundred and thirty-nine lines of header
/// advance, commit ordering, offset arithmetic, durable write and block seal —
/// and a copy of a durability path is a second place for a torn write to be
/// handled differently.
///
/// What is proved here is that the sidecar gets ALL of it: the commit counter
/// moves, the records land at the overlay's own 24-byte stride, and every field
/// comes back exactly as written — including both null sentinels, which is the
/// case that separates "the vendor stated nothing" from "the vendor stated
/// zero" for a value where zero is real.
#[test]
fn an_overlay_is_written_and_read_back_through_the_bar_writer() {
    use store::format::{OI_NULL, Overlay};

    let scratch = Scratch::new("overlaywrite");
    let path = StorePath::new(parts(FileKind::Overlay)).expect("a legal path");
    let mut file =
        BarFile::open_or_create(scratch.root(), path, SYMBOL).expect("the overlay opens");
    assert_eq!(file.records(), 0);

    let rows = [
        Overlay {
            ts_micros: T0, // inside the 2024-06 the path names (D-0915)
            spot: 2_465_005,
            iv_micros: 125_000,
        },
        // ONE OF EACH SENTINEL, because a vendor answering a spot without a
        // volatility is ordinary and the record must survive it.
        Overlay {
            ts_micros: T0 + MINUTE,
            spot: 2_465_100,
            iv_micros: OI_NULL,
        },
        // AND A GENUINE ZERO, which must NOT come back as absent. A deep
        // out-of-the-money option prints exactly this late in its life.
        Overlay {
            ts_micros: T0 + 2 * MINUTE,
            spot: 0,
            iv_micros: 0,
        },
    ];
    file.append(&rows).expect("the overlay batch lands");
    assert_eq!(file.records(), 3, "the commit counter moved");

    // REOPENED, so what is asserted is what reached the DISK rather than what
    // is still in the writer's own head.
    drop(file);
    let path = StorePath::new(parts(FileKind::Overlay)).expect("a legal path");
    let back = BarFile::open_or_create(scratch.root(), path, SYMBOL).expect("it reopens");
    assert_eq!(back.records(), 3, "and it survived the close");

    // AND A RERUN IS SAFE. CLAUDE.md §3 rule 5: the same batch appended twice
    // is accepted as already-stored rather than duplicated.
    let mut again = back;
    again.append(&rows).expect("a rerun is accepted");
    assert_eq!(
        again.records(),
        3,
        "the same three rows, not six — a rerun stores nothing new"
    );
}

// ===========================================================================
// audit-20261003: the writer's repair and creation doors
// ===========================================================================

/// attackdata-1 (D-1520). A month that HAD committed bars and was truncated
/// to nothing, or zeroed back to its header region, is refused by the writer
/// door, not silently rebuilt as an empty month.
///
/// The `.crc` sidecar beside it is the proof: it is written only by an append,
/// after the records and before the header slot, so a non-empty sidecar beside
/// a header region of zeros means records were committed and are gone. The
/// read door already refused the same file; the two doors now agree.
#[test]
fn a_truncated_month_whose_sidecar_proves_records_is_refused_not_reinitialised() {
    for truncated_to in [0_usize, 32_768] {
        let scratch = Scratch::new(&format!("truncated{truncated_to}"));
        let mut file = open(scratch.root()).expect("create");
        file.append(&batch(0, 3)).expect("three bars commit");
        drop(file);

        let on_disk = bars_path().to_path_buf(scratch.root());
        let sidecar = bars_path()
            .with_file(FileKind::Checksums)
            .to_path_buf(scratch.root());
        let sealed = fs::read(&sidecar).expect("the sidecar the append sealed");
        assert!(!sealed.is_empty(), "the premise: the append sealed a block");
        fs::write(&on_disk, vec![0u8; truncated_to]).expect("the truncation");

        let refused = outcome(open(scratch.root()));
        assert_eq!(
            refused,
            Err(StoreError::CommittedRecordsLost {
                path: on_disk.clone(),
                sidecar: sidecar.clone(),
                sidecar_len: u64::try_from(sealed.len()).expect("fits"),
            }),
            "{truncated_to}: a month whose sidecar proves committed records \
             must be refused by name, not reopened empty"
        );
        let text = refused.expect_err("refused").to_string();
        assert!(
            text.contains(&on_disk.display().to_string()) && text.contains("committed"),
            "the refusal names the file and why: {text}"
        );
        assert_eq!(
            fs::read(&on_disk).expect("the bar file").len(),
            truncated_to,
            "{truncated_to}: the refusal leaves the bar file as it was found"
        );
        assert_eq!(
            fs::read(&sidecar).expect("the sidecar"),
            sealed,
            "{truncated_to}: and the sidecar that proves the loss untouched"
        );
    }
}

/// D-1520's boundary. A month file DELETED whole, its sidecar left behind, is
/// created again as before: only a file that existed can have been truncated or
/// zeroed, and a deletion is an explicit act the refusal does not second-guess.
/// Stated in `docs/06-limits.md`.
#[test]
fn a_deleted_month_file_is_created_again_despite_its_sidecar() {
    let scratch = Scratch::new("deleted-month");
    let mut file = open(scratch.root()).expect("create");
    file.append(&batch(0, 3)).expect("three bars commit");
    drop(file);
    let on_disk = bars_path().to_path_buf(scratch.root());
    fs::remove_file(&on_disk).expect("the deletion");
    let reopened = open(scratch.root()).expect("a deleted month is created again");
    assert_eq!(reopened.header().n_valid, 0, "and it starts empty");
}

/// hunt-store-2 (D-1521). A genesis slot write that tore — the zero fill
/// landed and only part of the 64-byte slot did — leaves a file that cannot
/// hold a record and has no committed header. With no sidecar entry beside it
/// nothing was ever appended, so the writer repairs it exactly as it repairs
/// an all-zero region, instead of refusing it on every open forever.
#[test]
fn a_torn_genesis_slot_with_nothing_committed_is_repaired() {
    for torn_at in [1_usize, 8, 32, 63] {
        let scratch = Scratch::new(&format!("tornslot{torn_at}"));
        let on_disk = bars_path().to_path_buf(scratch.root());
        let dir = on_disk.parent().expect("a parent").to_path_buf();
        fs::create_dir_all(&dir).expect("the month directory");

        // The genesis image a real initialise writes, torn after `torn_at`
        // bytes of its first slot.
        let donor = Scratch::new(&format!("tornslotdonor{torn_at}"));
        drop(open(donor.root()).expect("a healthy empty month"));
        let healthy = image(donor.root());
        let mut torn = vec![0u8; 32_768];
        torn[..torn_at].copy_from_slice(&healthy[..torn_at]);
        fs::write(&on_disk, &torn).expect("the torn file");

        let opened = open(scratch.root()).unwrap_or_else(|why| {
            panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
        });
        assert_eq!(opened.records(), 0, "{torn_at}: repaired to an empty month");
        drop(opened);
        assert_eq!(
            image(scratch.root()),
            healthy,
            "{torn_at}: the repair writes the genesis region a fresh month has"
        );
    }
}

/// The other half of D-1520's proof: a sidecar that EXISTS but is EMPTY proves
/// nothing was committed, which is the state an append leaves when it creates
/// the `.crc` and dies before writing an entry. Beside a torn genesis slot it
/// must be repaired like an absent one, not refused for ever. P10-03: `> 0`
/// mutated to `>= 0` in `refuse_if_sealed` wedged exactly this month and no
/// test noticed, because every other fixture's sidecar is absent or non-empty.
#[test]
fn a_torn_genesis_slot_beside_an_empty_sidecar_is_repaired() {
    for torn_at in [1_usize, 63] {
        let scratch = Scratch::new(&format!("tornslotemptycrc{torn_at}"));
        let on_disk = bars_path().to_path_buf(scratch.root());
        let sidecar = bars_path()
            .with_file(FileKind::Checksums)
            .to_path_buf(scratch.root());
        fs::create_dir_all(on_disk.parent().expect("a parent")).expect("the month directory");

        let donor = Scratch::new(&format!("tornslotemptycrcdonor{torn_at}"));
        drop(open(donor.root()).expect("a healthy empty month"));
        let healthy = image(donor.root());
        let mut torn = vec![0u8; 32_768];
        torn[..torn_at].copy_from_slice(&healthy[..torn_at]);
        fs::write(&on_disk, &torn).expect("the torn file");
        fs::write(&sidecar, []).expect("the empty sidecar");
        assert_eq!(
            fs::metadata(&sidecar)
                .expect("the premise: it exists")
                .len(),
            0,
            "the premise: it is empty"
        );

        let opened = open(scratch.root()).unwrap_or_else(|why| {
            panic!(
                "{torn_at}: an empty sidecar proves nothing and the month must be repaired: {why}"
            )
        });
        assert_eq!(opened.records(), 0, "{torn_at}: repaired to an empty month");
        drop(opened);
        assert_eq!(
            image(scratch.root()),
            healthy,
            "{torn_at}: the repair writes the genesis region a fresh month has"
        );
    }
}

/// hunt-store-2's restraint: a slot that DECODES is a commit, not a tear, and
/// a file whose region holds anything outside the first slot is not the shape
/// a torn genesis can leave. Neither is re-initialised.
#[test]
fn a_region_with_bytes_a_torn_genesis_cannot_leave_is_still_refused() {
    let scratch = Scratch::new("tornslotfar");
    let on_disk = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(on_disk.parent().expect("a parent")).expect("the month directory");
    let mut damaged = vec![0u8; 32_768];
    damaged[0] = b'B';
    damaged[16_384 + 3] = 0x01;
    fs::write(&on_disk, &damaged).expect("the damaged file");
    assert!(
        open(scratch.root()).is_err(),
        "a byte in the second slot is not a torn genesis and must be refused"
    );
    assert_eq!(fs::read(&on_disk).expect("the file"), damaged, "untouched");
}

/// hunt-store-3 (D-1522). The writer does not recreate a missing store root.
///
/// A store on an unmounted volume would otherwise get a fresh root on the
/// parent filesystem and bars would land there, splitting append-only history
/// across two devices. The month's own directories below an existing root are
/// still created.
#[test]
fn a_missing_store_root_is_refused_not_recreated() {
    let scratch = Scratch::new("noroot");
    let root = scratch.root().join("unmounted");
    assert!(!root.exists(), "the premise: the root is absent");

    let refused = outcome(BarFile::open_or_create(&root, bars_path(), SYMBOL));
    assert_eq!(
        refused,
        Err(StoreError::Missing {
            path: root.clone(),
            action: Action::Open,
        }),
        "a missing store root is refused by name"
    );
    assert!(!root.exists(), "and it was not created");

    // Below an existing root, the month's directories are still made.
    fs::create_dir_all(&root).expect("the root, mounted");
    assert_eq!(
        outcome(BarFile::open_or_create(&root, bars_path(), SYMBOL)),
        Ok(0)
    );
}

/// hunt-store-1 (D-1523). The ordinary bar door refuses an overlay file put at
/// a `.bin` name instead of serving its 24-byte records as 56-byte bars.
#[test]
fn the_bar_door_refuses_an_overlay_or_greeks_file_at_a_bar_name() {
    use store::format::{Greek, Overlay};

    for kind in [FileKind::Overlay, FileKind::Greeks] {
        let scratch = Scratch::new(&format!("wronggeometry{kind:?}"));
        let path = StorePath::new(parts(kind)).expect("a legal path");
        let mut file = BarFile::open_or_create(scratch.root(), path, SYMBOL).expect("opens");
        let version = if kind == FileKind::Overlay {
            let rows: Vec<Overlay> = (0..5)
                .map(|index| Overlay {
                    ts_micros: T0 + index * MINUTE,
                    spot: 2_310_955,
                    iv_micros: 125_000,
                })
                .collect();
            file.append(&rows).expect("overlay rows");
            Layout::OVERLAY.version()
        } else {
            let rows: Vec<Greek> = (0..5)
                .map(|index| Greek {
                    ts_micros: T0 + index * MINUTE,
                    spot: 2_400_000,
                    volatility: 0.25,
                    delta: 0.5,
                    gamma: 0.125,
                    vega: 1.0,
                    theta: -1.0,
                    rho: 2.0,
                    rate: 0.0,
                    provenance: 0,
                })
                .collect();
            file.append(&rows).expect("greeks rows");
            Layout::GREEKS.version()
        };
        drop(file);

        let sidecar_kind = kind.checksums().expect("a record kind has a sidecar");
        fs::copy(
            path.to_path_buf(scratch.root()),
            bars_path().to_path_buf(scratch.root()),
        )
        .expect("the file at a bar name");
        fs::copy(
            path.with_file(sidecar_kind).to_path_buf(scratch.root()),
            bars_path()
                .with_file(FileKind::Checksums)
                .to_path_buf(scratch.root()),
        )
        .expect("its sidecar at the bar sidecar's name");

        for refused in [
            outcome(BarFile::open_existing(scratch.root(), bars_path(), SYMBOL)),
            outcome(BarFile::open_or_create(scratch.root(), bars_path(), SYMBOL)),
        ] {
            assert_eq!(
                refused,
                Err(StoreError::Format {
                    path: bars_path().to_path_buf(scratch.root()),
                    source: FormatError::UnknownVersion(version),
                }),
                "{kind:?}: a bar path is offered only the bar geometries"
            );
        }
    }
}

/// attackdata-2 (D-1524). A greeks re-run is compared byte for byte, not by
/// `f64` equality: a row whose delta is `-0.0` where the held row's is `0.0`
/// has different bytes, so it is not "already present" — it is a restatement,
/// refused by name.
#[test]
fn a_greek_rerun_with_a_negative_zero_is_not_already_present() {
    use store::format::Greek;

    let scratch = Scratch::new("greekzero");
    let path = StorePath::new(parts(FileKind::Greeks)).expect("a legal path");
    let mut file = BarFile::open_or_create(scratch.root(), path, SYMBOL).expect("opens");
    let held = Greek {
        ts_micros: T0,
        spot: 2_400_000,
        volatility: 0.25,
        delta: 0.0,
        gamma: 0.125,
        vega: 1.0,
        theta: -1.0,
        rho: 2.0,
        rate: 0.0,
        provenance: 0,
    };
    file.append(&[held]).expect("the held row");
    let rerun = Greek {
        delta: -0.0,
        ..held
    };
    assert_ne!(held.image(), rerun.image(), "the premise: the bytes differ");
    assert_eq!(
        file.append(&[rerun]),
        Err(StoreError::OverlapDisagrees {
            path: path.to_path_buf(scratch.root()),
            at: 0,
            ts_micros: T0,
            conflict: Conflict::Restated,
        }),
        "different bytes are not the bytes already stored"
    );
    assert_eq!(
        file.append(&[held]),
        Ok(Appended::AlreadyPresent {
            first_index: 0,
            n_valid: 1
        }),
        "and the identical bytes still are"
    );
}

/// attackdata-7 (D-1525). A batch whose overlap disagrees with what the month
/// holds is refused with the real diagnosis, not as "timestamps out of order":
/// a held bar restated with different values, a bar at a stamp inside the held
/// range that the month never held, and a batch that skips a held bar.
#[test]
fn an_overlap_that_disagrees_is_refused_with_the_real_diagnosis() {
    let scratch = Scratch::new("diagnosis");
    let mut file = open(scratch.root()).expect("create");
    // Held: minutes 0, 2, 4, 6, 8 — a month with holes, so a stamp inside the
    // held range can be absent.
    let held: Vec<Bar> = [0, 2, 4, 6, 8].into_iter().map(bar).collect();
    file.append(&held).expect("held");
    let on_disk = bars_path().to_path_buf(scratch.root());
    let before = image(scratch.root());

    // A restatement: minute 4 offered with a different close.
    let mut restated = vec![bar(4), bar(6), bar(8), bar(9)];
    restated[0].close += 1;
    assert_eq!(
        file.append(&restated),
        Err(StoreError::OverlapDisagrees {
            path: on_disk.clone(),
            at: 0,
            ts_micros: T0 + 4 * MINUTE,
            conflict: Conflict::Restated,
        })
    );

    // A stamp the month never held: minute 5 sits between held 4 and 6.
    assert_eq!(
        file.append(&[bar(4), bar(5), bar(6), bar(8), bar(9)]),
        Err(StoreError::OverlapDisagrees {
            path: on_disk.clone(),
            at: 1,
            ts_micros: T0 + 5 * MINUTE,
            conflict: Conflict::NotHeld,
        })
    );

    // A batch that skips held minute 6, naming the held stamp it skipped.
    assert_eq!(
        file.append(&[bar(4), bar(8), bar(9)]),
        Err(StoreError::OverlapDisagrees {
            path: on_disk.clone(),
            at: 1,
            ts_micros: T0 + 6 * MINUTE,
            conflict: Conflict::Skipped,
        })
    );
    let text = StoreError::OverlapDisagrees {
        path: on_disk,
        at: 0,
        ts_micros: T0,
        conflict: Conflict::Restated,
    }
    .to_string();
    assert!(
        text.contains("restate"),
        "the message names the restatement: {text}"
    );
    assert_eq!(image(scratch.root()), before, "and nothing was written");
}

// ===========================================================================
// A header is checked against its month (CE-61, CE-63, CE-62)
// ===========================================================================

/// Ten real bars, then a newer header slot carrying `forge`'s changes and a
/// file long enough for whatever counter it names. The slot is CRC-valid: it
/// is written through the same public `Header::commit` the writer uses.
fn forged(tag: &str, forge: impl Fn(&mut Header)) -> (Scratch, PathBuf) {
    use std::os::unix::fs::FileExt as _;
    let scratch = Scratch::new(tag);
    let mut header = {
        let mut file = open(scratch.root()).expect("create");
        assert!(file.append(&batch(0, 10)).is_ok());
        file.header()
    };
    header.generation += 1;
    forge(&mut header);
    let commit = header.commit().expect("a committable forged header");
    let bars = bars_path().to_path_buf(scratch.root());
    let handle = fs::OpenOptions::new().write(true).open(&bars).unwrap();
    handle
        .set_len(HEADER_LEN + header.n_valid * RECORD_STRIDE)
        .unwrap();
    handle.write_at(&commit.bytes, commit.offset).unwrap();
    drop(handle);
    (scratch, bars)
}

/// **CE-61. A COUNTER PAST WHAT THE MONTH CAN HOLD IS REFUSED AT OPEN.**
///
/// The only bound on `n_valid` was the file's length, so a sparse file and a
/// CRC-valid slot opened, and every reader that sized a vector or a loop from
/// the counter did so from a number the writer could never have committed:
/// the api aborted on the allocation. June at one minute holds 43,200 grid
/// slots; 43,201 is refused, by name, at both doors.
#[test]
fn a_header_counting_more_records_than_its_month_holds_is_refused() {
    let (scratch, bars) = forged("counter-past-month", |header| header.n_valid = 43_201);
    let refused = Err(StoreError::Format {
        path: bars.clone(),
        source: FormatError::CounterExceedsMonth {
            n_valid: 43_201,
            slots: 43_200,
        },
    });
    assert_eq!(outcome(open(scratch.root())), refused);
    assert_eq!(
        outcome(BarFile::open_existing(scratch.root(), bars_path(), SYMBOL)),
        refused,
        "the read door refuses the same file"
    );

    // EXACTLY THE MONTH IS LEGAL: the bound is the grid, not a guess below it.
    let (full, _) = forged("counter-at-month", |header| header.n_valid = 43_200);
    assert_eq!(
        outcome(BarFile::open_existing(full.root(), bars_path(), SYMBOL)),
        Ok(43_200)
    );
}

/// **CE-63. A HEADER WHOSE RANGE LEAVES ITS MONTH IS REFUSED AS A HEADER
/// FAULT,** not discovered later as an overlap that names the wrong batch.
#[test]
fn a_header_whose_timestamps_leave_its_month_is_refused() {
    // 2034-06-03, ten years past the month, and 2024-05-31, the day before it.
    let late = T0 + 3_652 * 86_400_000_000;
    let early = T0 - 3 * 86_400_000_000;
    for (tag, first, last) in [
        ("range-late", T0, late),
        ("range-early", early, T0 + 9 * MINUTE),
    ] {
        let (scratch, bars) = forged(tag, |header| {
            header.first_ts_micros = first;
            header.last_ts_micros = last;
        });
        assert_eq!(
            outcome(BarFile::open_existing(scratch.root(), bars_path(), SYMBOL)),
            Err(StoreError::Format {
                path: bars,
                source: FormatError::RangeOutsideMonth {
                    first_ts_micros: first,
                    last_ts_micros: last,
                },
            }),
            "{tag}"
        );
    }
}

/// **CE-62. THE WRITER DOES NOT FOLLOW A SYMLINK** at the month file or at a
/// directory below the store root: an append through one lands in whatever
/// file the link names, another vendor's month among them. Refused by the
/// linked component's name, and nothing is written through it.
#[test]
fn the_writer_refuses_a_symlinked_month_file_or_directory() {
    let scratch = Scratch::new("symlink-leaf");
    let elsewhere = scratch.root().join("elsewhere.bin");
    fs::write(&elsewhere, b"").unwrap();
    let bars = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(bars.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &bars).unwrap();
    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Symlinked { path: bars.clone() })
    );
    assert_eq!(
        fs::read(&elsewhere).unwrap(),
        b"",
        "nothing written through it"
    );

    let scratch = Scratch::new("symlink-dir");
    let real = scratch.root().join("real-month");
    fs::create_dir_all(&real).unwrap();
    let bars = bars_path().to_path_buf(scratch.root());
    let month_dir = bars.parent().unwrap().to_path_buf();
    fs::create_dir_all(month_dir.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&real, &month_dir).unwrap();
    assert_eq!(
        outcome(open(scratch.root())),
        Err(StoreError::Symlinked { path: month_dir })
    );
    assert_eq!(
        fs::read_dir(&real).unwrap().count(),
        0,
        "nothing created through the linked directory"
    );
}
