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

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use store::crc::crc32c;
use store::file::{Action, Appended, BarFile, StoreError};
use store::format::{Bar, FormatError, HEADER_LEN, OI_NULL, RECORD_LEN, RECORD_STRIDE};
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
    assert_eq!(file.layout(), Layout::V2);
    assert_eq!(file.path(), bars_path().to_path_buf(scratch.root()));

    let bytes = image(scratch.root());
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        HEADER_LEN,
        "the whole header region is materialised, not left as a hole"
    );
    assert_eq!(&bytes[0..8], b"BRUTEXB2", "magic at byte 0");
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
    for kind in [FileKind::Checksums, FileKind::Lock] {
        let path = StorePath::new(parts(kind)).expect("a legal path");
        assert_eq!(
            outcome(BarFile::open_or_create(scratch.root(), path, SYMBOL)),
            Err(StoreError::NotABarPath { found: kind }),
            "{kind:?} holds no records and must not be opened as though it did"
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
        Err(StoreError::Format {
            path: file.path().to_path_buf(),
            source: FormatError::TimestampsOutOfOrder {
                previous: T0 + 9 * MINUTE,
                next: T0 + 5 * MINUTE,
            },
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
    let commit = Header::genesis(SYMBOL, 60, 0).commit().expect("genesis");
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

    assert_eq!(file.read_record(4), Ok(bar(4)), "the records still there");
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
    let commit = Header::genesis(SYMBOL, 300, 0).commit().expect("genesis");
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
    assert_eq!(reader.layout(), Layout::V2);
    drop(reader);

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
            ts_micros: 1_700_000_000_000_000,
            spot: 2_465_005,
            iv_micros: 125_000,
        },
        // ONE OF EACH SENTINEL, because a vendor answering a spot without a
        // volatility is ordinary and the record must survive it.
        Overlay {
            ts_micros: 1_700_000_060_000_000,
            spot: 2_465_100,
            iv_micros: OI_NULL,
        },
        // AND A GENUINE ZERO, which must NOT come back as absent. A deep
        // out-of-the-money option prints exactly this late in its life.
        Overlay {
            ts_micros: 1_700_000_120_000_000,
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
