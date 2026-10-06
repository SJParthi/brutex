//! The `.tix` time index through real files — D-2329, D-2330.
//!
//! `store::time_index` proves the lookup's read count on indexes built in
//! memory. This file proves the other half: that the index the WRITER keeps
//! on disk answers exactly what the D-1434 bisection answers on the same
//! month, that a month without one bisects and says why, that a writer open
//! builds one, and that every way an index can be wrong is noticed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::time_index::Why;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-tix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn tix(&self, timeframe: Timeframe) -> PathBuf {
        path(timeframe)
            .with_file(FileKind::TimeIndex)
            .to_path_buf(&self.0)
    }

    fn writer(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_or_create(&self.0, path(timeframe), 7).unwrap()
    }

    fn reader(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_existing(&self.0, path(timeframe), 7).unwrap()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// 2024-06-01 00:00 IST.
const JUNE_START: i64 = 1_717_180_200_000_000;
const MICROS: i64 = 1_000_000;

/// 09:15 IST on day `day` of June 2024, plus `secs`.
fn at(day: i64, secs: i64) -> i64 {
    JUNE_START + day * 86_400 * MICROS + (555 * 60 + secs) * MICROS
}

fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 2_000_000,
        high: 2_000_100,
        low: 1_999_900,
        close: 2_000_050,
        volume: 10,
        open_interest: OI_NULL,
    }
}

/// `days` weekdays of `per_day` bars `step` seconds apart from 09:15 IST,
/// with every 37th bar of a session left out, so the month has gaps inside
/// sessions as well as between them.
fn month_of(days: i64, per_day: i64, step: i64) -> Vec<Bar> {
    let mut out = Vec::new();
    let mut day = 0;
    let mut taken = 0;
    while taken < days {
        if day % 7 >= 2 {
            for k in (0..per_day).filter(|k| k % 37 != 36) {
                out.push(bar(at(day, k * step)));
            }
            taken += 1;
        }
        day += 1;
    }
    out
}

/// Every probe the comparison below asks: each bar's stamp, a microsecond
/// either side, the middle of every gap, and both ends of the month.
fn probes(bars: &[Bar]) -> Vec<i64> {
    let (from, until) = YearMonth::new(2024, 6).unwrap().ist_bounds_micros();
    let mut out = vec![i64::MIN, 0, from - 1, from, until - 1, until, i64::MAX];
    for pair in bars.windows(2) {
        let (a, b) = (pair[0].ts_micros, pair[1].ts_micros);
        out.extend([a - 1, a, a + 1, a + (b - a) / 2]);
    }
    if let Some(last) = bars.last() {
        out.extend([last.ts_micros - 1, last.ts_micros, last.ts_micros + 1]);
    }
    out
}

/// Writes `bars` through the writer in batches of `batch`.
fn write(root: &Root, timeframe: Timeframe, bars: &[Bar], batch: usize) {
    let mut writer = root.writer(timeframe);
    for chunk in bars.chunks(batch) {
        assert!(matches!(
            writer.append(chunk),
            Ok(Appended::Committed { .. })
        ));
    }
}

#[test]
fn the_index_answers_every_timestamp_exactly_as_the_bisection_does() {
    // D-2329. The same committed month, asked every probe twice: once through
    // its index and once with the index taken away, which is the D-1434
    // bisection unchanged. Every answer must agree, at the one-minute and the
    // one-second rung, written in batches that end mid-bucket and cross gaps.
    for (timeframe, bars, batch) in [
        (Timeframe::MINUTE_1, month_of(20, 375, 60), 101),
        (Timeframe::SECOND_1, month_of(2, 22_500, 1), 4_999),
        (Timeframe::MINUTE_5, month_of(10, 75, 300), 7),
    ] {
        let root = Root::new();
        write(&root, timeframe, &bars, batch);
        let indexed = root.reader(timeframe);
        assert_eq!(indexed.time_lookup(), TimeLookup::Indexed);
        let answers: Vec<u64> = probes(&bars)
            .iter()
            .map(|&ts| indexed.first_at_or_after(ts).unwrap())
            .collect();
        drop(indexed);

        fs::remove_file(root.tix(timeframe)).unwrap();
        let bisecting = root.reader(timeframe);
        assert_eq!(bisecting.time_lookup(), TimeLookup::Bisection(Why::Absent));
        for (ts, want) in probes(&bars).into_iter().zip(answers) {
            assert_eq!(
                bisecting.first_at_or_after(ts).unwrap(),
                want,
                "{}: ts={ts}",
                timeframe.as_str()
            );
            // And both are the definition: bars stamped before `ts`.
            let truth = bars.partition_point(|b| b.ts_micros < ts) as u64;
            assert_eq!(want, truth, "{}: ts={ts}", timeframe.as_str());
        }
    }
}

#[test]
fn a_month_without_an_index_bisects_and_a_writer_open_builds_one_once() {
    let root = Root::new();
    let tf = Timeframe::MINUTE_1;
    let bars = month_of(3, 375, 60);
    write(&root, tf, &bars, 500);
    let built = fs::read(root.tix(tf)).unwrap();
    fs::remove_file(root.tix(tf)).unwrap();

    // A reader creates nothing: the month bisects and says so.
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
    assert_eq!(reader.first_at_or_after(bars[400].ts_micros), Ok(400));
    drop(reader);
    assert!(!root.tix(tf).exists(), "a read door never writes the index");

    // The writer door rebuilds it, byte for byte what the appends kept.
    drop(root.writer(tf));
    assert_eq!(fs::read(root.tix(tf)).unwrap(), built);
    assert_eq!(root.reader(tf).time_lookup(), TimeLookup::Indexed);

    // A second writer open finds it good and writes nothing.
    let before = fs::metadata(root.tix(tf)).unwrap().modified().unwrap();
    drop(root.writer(tf));
    assert_eq!(fs::read(root.tix(tf)).unwrap(), built);
    assert_eq!(
        fs::metadata(root.tix(tf)).unwrap().modified().unwrap(),
        before
    );
}

#[test]
fn the_index_file_is_the_documented_bytes() {
    // docs/02-store-format.md §8.1: a 64-byte header, then one 16-byte entry
    // per 64 slots up to the bucket of the last bar.
    let root = Root::new();
    let tf = Timeframe::MINUTE_1;
    // Day 2 09:15 is minute 2,880 + 555 = 3,435 of the month: bucket 53, bit 43.
    let bars = vec![bar(at(2, 0)), bar(at(2, 60)), bar(at(2, 21 * 60))];
    write(&root, tf, &bars, 3);
    let raw = fs::read(root.tix(tf)).unwrap();
    // Minute 3,435 + 21 = 3,456 = bucket 54, bit 0.
    assert_eq!(raw.len(), 64 + 55 * 16);
    assert_eq!(&raw[..8], b"BRUTEXT1");
    let entry = |b: usize| {
        let e = &raw[64 + b * 16..64 + b * 16 + 16];
        (
            u64::from_le_bytes(e[..8].try_into().unwrap()),
            u32::from_le_bytes(e[8..12].try_into().unwrap()),
        )
    };
    assert_eq!(entry(0), (0, 0));
    assert_eq!(entry(52), (0, 0));
    assert_eq!(entry(53), ((1 << 43) | (1 << 44), 0));
    assert_eq!(entry(54), (1, 2));

    // An empty month is the header alone.
    let empty = Root::new();
    drop(empty.writer(tf));
    assert_eq!(fs::read(empty.tix(tf)).unwrap().len(), 64);
}

#[test]
fn a_second_daily_bar_on_one_ist_day_is_admitted_and_the_month_stops_being_indexed() {
    // D-2330. The daily rung admits any whole second (D-0915) and the index
    // holds one bar per slot. The append is admitted exactly as before D-2329;
    // the `.tix` is removed before any byte moves, and lookups bisect, saying
    // why — across a commit and within one batch.
    let tf = Timeframe::DAY_1;
    let root = Root::new();
    let mut writer = root.writer(tf);
    assert!(writer.append(&[bar(at(3, 0))]).is_ok());
    assert!(root.tix(tf).exists());
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    assert!(writer.append(&[bar(at(3, 3_600))]).is_ok(), "admitted");
    assert!(
        !root.tix(tf).exists(),
        "no index is left claiming the month"
    );
    let shared = Why::SharedSlot {
        index: 1,
        ts_micros: at(3, 3_600),
    };
    assert_eq!(writer.time_lookup(), TimeLookup::Bisection(shared.clone()));
    assert!(writer.append(&[bar(at(4, 0)), bar(at(4, 60))]).is_ok());
    assert!(!root.tix(tf).exists());
    let expected = [
        (at(3, -1), 0),
        (at(3, 0), 0),
        (at(3, 1), 1),
        (at(3, 3_600), 1),
        (at(3, 3_601), 2),
        (at(4, 0), 2),
        (at(4, 30), 3),
        (at(4, 60), 3),
        (at(4, 61), 4),
    ];
    for (ts, row) in expected {
        assert_eq!(writer.first_at_or_after(ts), Ok(row), "writer, ts {ts}");
    }
    drop(writer);
    // A writer open tries the rebuild, meets the same bar, and says so again.
    let reopened = root.writer(tf);
    assert!(!root.tix(tf).exists());
    assert_eq!(reopened.time_lookup(), TimeLookup::Bisection(shared));
    drop(reopened);
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
    for (ts, row) in expected {
        assert_eq!(reader.first_at_or_after(ts), Ok(row), "reader, ts {ts}");
    }

    // Within one batch, on a fresh month.
    let root = Root::new();
    let mut writer = root.writer(tf);
    assert!(writer.append(&[bar(at(4, 0)), bar(at(4, 60))]).is_ok());
    assert!(!root.tix(tf).exists());
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::SharedSlot {
            index: 1,
            ts_micros: at(4, 60),
        })
    );
    // One bar per IST day keeps its index: a bar at the next day's midnight
    // is a new slot.
    let root = Root::new();
    let mut writer = root.writer(tf);
    assert!(writer.append(&[bar(at(3, 0))]).is_ok());
    assert!(writer.append(&[bar(at(4, -555 * 60))]).is_ok());
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    assert_eq!(writer.first_at_or_after(at(4, -555 * 60 - 1)), Ok(1));
    assert_eq!(writer.first_at_or_after(at(4, -555 * 60)), Ok(1));
    assert_eq!(writer.first_at_or_after(at(4, -555 * 60 + 1)), Ok(2));
    assert_eq!(writer.first_at_or_after(at(3, 1)), Ok(1));
}

#[test]
fn an_index_left_ahead_of_the_commit_is_masked_and_then_overwritten() {
    // A crash after the index write and before the header slot leaves entries
    // for bars that never committed. Built here by keeping a longer month's
    // index beside a shorter month's bars: the committed prefix is identical.
    let tf = Timeframe::MINUTE_1;
    let all = month_of(2, 375, 60);
    let committed = &all[..200];
    let longer = Root::new();
    write(&longer, tf, &all, 1_000);
    let ahead = fs::read(longer.tix(tf)).unwrap();

    let root = Root::new();
    write(&root, tf, committed, 1_000);
    fs::write(root.tix(tf), &ahead).unwrap();

    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    for ts in probes(committed) {
        let truth = committed.partition_point(|b| b.ts_micros < ts) as u64;
        assert_eq!(reader.first_at_or_after(ts), Ok(truth), "ts={ts}");
    }
    drop(reader);

    // The next append writes DIFFERENT bars past the commit; the index
    // follows the commit, not the leftovers.
    let other: Vec<Bar> = (0..30).map(|k| bar(at(9, k * 60))).collect();
    write(&root, tf, &other, 30);
    let mut now = committed.to_vec();
    now.extend(other);
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    for ts in probes(&now) {
        let truth = now.partition_point(|b| b.ts_micros < ts) as u64;
        assert_eq!(reader.first_at_or_after(ts), Ok(truth), "ts={ts}");
    }
}

#[test]
fn every_way_an_index_can_be_wrong_is_named_and_the_answer_still_comes_back() {
    let tf = Timeframe::MINUTE_1;
    let bars = month_of(3, 375, 60);
    let probe = bars[600].ts_micros + 1;

    // Stale: the month grew without its index, by one bar in the same
    // bucket — the entry is there and does not hold the last bar.
    let small = Root::new();
    write(&small, tf, &bars[..2], 2);
    let old = fs::read(small.tix(tf)).unwrap();
    write(&small, tf, &bars[2..3], 1);
    fs::write(small.tix(tf), &old).unwrap();
    let reader = small.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Stale));
    assert_eq!(reader.first_at_or_after(bars[1].ts_micros + 1), Ok(2));
    drop(reader);

    // Grown past the index's last entry: the entry for the last bar is
    // missing altogether.
    let root = Root::new();
    write(&root, tf, &bars[..500], 500);
    let old = fs::read(root.tix(tf)).unwrap();
    write(&root, tf, &bars[500..], 500);
    fs::write(root.tix(tf), &old).unwrap();
    let reader = root.reader(tf);
    assert!(matches!(
        reader.time_lookup(),
        TimeLookup::Bisection(Why::Entry { .. })
    ));
    assert_eq!(reader.first_at_or_after(probe), Ok(601));
    drop(reader);

    // A damaged header, a foreign geometry and a file shorter than a header.
    let mut damaged = fs::read(root.tix(tf)).unwrap();
    damaged[20] ^= 1;
    for (bytes, why) in [
        (damaged, Why::Header("its header fails its checksum")),
        (vec![0u8; 10], Why::Header("it is shorter than its header")),
    ] {
        fs::write(root.tix(tf), &bytes).unwrap();
        let reader = root.reader(tf);
        assert_eq!(reader.time_lookup(), TimeLookup::Bisection(why));
        assert_eq!(reader.first_at_or_after(probe), Ok(601));
    }

    // The writer repairs every one of them, and says so in the log.
    drop(root.writer(tf));
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    assert_eq!(reader.first_at_or_after(probe), Ok(601));
    drop(reader);

    // An entry damaged AFTER the handle confirmed the index: that one lookup
    // bisects, and the answer is the same.
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    let mut raw = fs::read(root.tix(tf)).unwrap();
    let slot = (bars[600].ts_micros - JUNE_START) / (60 * MICROS);
    let entry = 64 + usize::try_from(slot / 64).unwrap() * 16;
    raw[entry] ^= 0xFF;
    write_in_place(&root.tix(tf), &raw);
    assert_eq!(reader.first_at_or_after(probe), Ok(601));
}

/// Overwrites `path` without replacing its inode, so an open handle sees it.
fn write_in_place(path: &Path, bytes: &[u8]) {
    use std::os::unix::fs::FileExt as _;
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.write_all_at(bytes, 0).unwrap();
}

#[test]
fn a_month_whose_bars_cannot_be_read_keeps_no_index_and_still_opens() {
    // The rebuild reads every bar through the verified path. A block that
    // fails its checksum fails the rebuild: the writer door still opens the
    // month — as it did before D-2329 — leaves no `.tix` claiming bars it
    // could not read, and bisects, saying why.
    let root = Root::new();
    let tf = Timeframe::MINUTE_1;
    let bars = month_of(1, 375, 60);
    write(&root, tf, &bars, 375);
    fs::remove_file(root.tix(tf)).unwrap();
    let bin = path(tf).to_path_buf(&root.0);
    let mut raw = fs::read(&bin).unwrap();
    raw[32_768 + 8] ^= 1;
    fs::write(&bin, &raw).unwrap();

    let writer = root.writer(tf);
    assert!(matches!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::Unreadable(StoreError::BlockChecksum { .. }))
    ));
    assert!(!root.tix(tf).exists());
    // A lookup whose bisection never reaches the damaged block is answered.
    assert_eq!(writer.first_at_or_after(i64::MAX), Ok(365));
}

#[test]
fn a_rerun_of_the_same_month_leaves_the_index_byte_identical() {
    // CLAUDE.md §3 rule 5. Re-offering what is held is `AlreadyPresent`, and
    // neither the bars nor the index move; the same month written twice from
    // nothing is the same index.
    let tf = Timeframe::MINUTE_1;
    let bars = month_of(2, 375, 60);
    let first = Root::new();
    write(&first, tf, &bars, 97);
    let index = fs::read(first.tix(tf)).unwrap();
    let mut writer = first.writer(tf);
    assert!(matches!(
        writer.append(&bars[100..300]),
        Ok(Appended::AlreadyPresent {
            first_index: 100,
            ..
        })
    ));
    drop(writer);
    assert_eq!(fs::read(first.tix(tf)).unwrap(), index);

    let second = Root::new();
    write(&second, tf, &bars, 750);
    assert_eq!(fs::read(second.tix(tf)).unwrap(), index);
}

#[test]
fn only_a_bar_file_carries_a_time_index() {
    let root = Root::new();
    let overlay = path(Timeframe::MINUTE_1).with_file(FileKind::Overlay);
    let file = BarFile::open_or_create(&root.0, overlay, 7).unwrap();
    assert_eq!(file.time_lookup(), TimeLookup::Bisection(Why::NotBars));
    assert!(!root.tix(Timeframe::MINUTE_1).exists());
    assert_eq!(FileKind::TimeIndex.extension(), ".tix");
    assert_eq!(FileKind::TimeIndex.checksums(), None);
    assert!(matches!(
        BarFile::open_existing(
            &root.0,
            path(Timeframe::MINUTE_1).with_file(FileKind::TimeIndex),
            7
        ),
        Err(StoreError::NotABarPath {
            found: FileKind::TimeIndex
        })
    ));
}

/// **AN INDEX THAT IS THERE AND CANNOT BE OPENED IS UNREADABLE, NOT ABSENT.**
/// G18-rest-24, D-2077.
///
/// `Why::Absent` tells an operator to open a writer, which builds an index; a
/// directory where the `.tix` belongs is not fixed by that and must be named
/// as what it is. Only a host `NotFound` is absence.
#[test]
fn an_index_that_cannot_be_opened_is_named_unreadable_not_absent() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    let bars = month_of(1, 10, 60);
    write(&root, tf, &bars, 10);
    fs::remove_file(root.tix(tf)).unwrap();
    fs::create_dir(root.tix(tf)).unwrap();
    let reader = root.reader(tf);
    let lookup = reader.time_lookup();
    assert!(
        matches!(
            lookup,
            TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { .. }))
        ),
        "{lookup:?}"
    );
    assert_eq!(reader.first_at_or_after(bars[4].ts_micros), Ok(4));
}

/// **A WRITER OPEN OF AN EMPTY MONTH KEEPS ONLY THE FRESH INDEX.**
/// G18-rest-25, D-2077.
///
/// The fresh index is exactly its header, and that header confirms. An index
/// that confirms with bytes past its header, and one of the header's length
/// whose header does not confirm, are each rewritten to the fresh bytes: the
/// length test and the confirmation are BOTH required, and neither alone.
#[test]
fn a_writer_open_of_an_empty_month_rewrites_every_index_but_the_fresh_one() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    drop(root.writer(tf));
    let fresh = fs::read(root.tix(tf)).unwrap();
    assert_eq!(fresh.len() as u64, store::time_index::HEADER_LEN);

    let mut long = fresh.clone();
    long.extend_from_slice(&[0u8; 16]);
    let mut damaged = fresh.clone();
    damaged[20] ^= 1;
    for (bytes, what) in [
        (long, "bytes past the header"),
        (damaged, "a damaged header"),
    ] {
        fs::write(root.tix(tf), &bytes).unwrap();
        drop(root.writer(tf));
        assert_eq!(fs::read(root.tix(tf)).unwrap(), fresh, "{what}");
    }
}

/// **AN INDEX DAMAGED UNDER A LIVE WRITER IS REBUILT BEFORE ITS NEXT APPEND.**
/// G18-rest-26, D-2077.
///
/// The writer confirmed its index at open; the index is then cut back to its
/// header under it. The next append cannot resume from the cut entry, so the
/// writer rebuilds the index from its bars and the append commits. Without the
/// rebuild the second resume fails the same way and the append is refused.
#[test]
fn an_index_damaged_under_a_live_writer_is_rebuilt_before_its_append() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    let bars = month_of(2, 30, 60);
    let half = bars.len() / 2;
    let mut writer = root.writer(tf);
    assert!(matches!(
        writer.append(&bars[..half]),
        Ok(Appended::Committed { .. })
    ));
    fs::OpenOptions::new()
        .write(true)
        .open(root.tix(tf))
        .unwrap()
        .set_len(store::time_index::HEADER_LEN)
        .unwrap();
    assert!(matches!(
        writer.append(&bars[half..]),
        Ok(Appended::Committed { .. })
    ));
    drop(writer);
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    for (row, bar) in bars.iter().enumerate() {
        assert_eq!(reader.first_at_or_after(bar.ts_micros), Ok(row as u64));
    }
}
