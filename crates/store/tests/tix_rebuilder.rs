//! `BarFile::index_existing` — the `.tix` a named month gets through the
//! writer's index step, with none of a writer's other effects. D-3340, WP-15;
//! DB-064, DB-G81.
//!
//! What each test holds, in the order an operator meets it: a month written
//! before D-2329 gets the index an appending writer would have kept, byte for
//! byte, and nothing else on disk moves; a second run writes nothing; a month
//! that is not there is refused with nothing created; a month another holder
//! has is refused and not written; a month no index can describe is named and
//! left without one; a stale, damaged or empty month's index is rebuilt and
//! says why; and a link below the root, a wrong symbol or a sidecar path is
//! refused before anything is written.
//!
//! Every bar is invented: one-second stamps on a 2019-03 grid at a price no
//! index ever printed.

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
use store::file::{Appended, BarFile, IndexedMonth, Reindexed, StoreError, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::time_index::Why;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The symbol id every month here is written with.
const SYMBOL: u32 = 7;

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-tix-rebuild-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, timeframe: Timeframe, kind: FileKind) -> PathBuf {
        path(timeframe).with_file(kind).to_path_buf(&self.0)
    }

    fn tix(&self, timeframe: Timeframe) -> PathBuf {
        self.file(timeframe, FileKind::TimeIndex)
    }

    fn writer(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_or_create(&self.0, path(timeframe), SYMBOL).unwrap()
    }

    fn reader(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_existing(&self.0, path(timeframe), SYMBOL).unwrap()
    }

    fn index(&self, timeframe: Timeframe) -> Result<IndexedMonth, StoreError> {
        BarFile::index_existing(&self.0, path(timeframe), SYMBOL)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month() -> YearMonth {
    YearMonth::new(2019, 3).unwrap()
}

fn path(timeframe: Timeframe) -> StorePath<'static> {
    path_at(timeframe, month(), FileKind::Bars)
}

fn path_at(timeframe: Timeframe, month: YearMonth, file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month,
        file,
    })
    .unwrap()
}

const MICROS: i64 = 1_000_000;

/// 09:15 IST on day `day` (0-based) of March 2019, plus `secs`.
fn at(day: i64, secs: i64) -> i64 {
    month().ist_bounds_micros().0 + day * 86_400 * MICROS + (555 * 60 + secs) * MICROS
}

fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 4_200_000,
        high: 4_200_100,
        low: 4_199_900,
        close: 4_200_050,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// `days` sessions of `per_day` bars `step` seconds apart from 09:15 IST, every
/// 37th left out so the month has holes inside sessions as well as between.
fn sessions(days: i64, per_day: i64, step: i64) -> Vec<Bar> {
    let mut out = Vec::new();
    for day in 0..days {
        for k in (0..per_day).filter(|k| k % 37 != 36) {
            out.push(bar(at(day * 2, k * step)));
        }
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

/// Every probe worth asking: each stamp, a microsecond either side, the
/// middle of every gap, and both ends of the month.
fn probes(bars: &[Bar]) -> Vec<i64> {
    let (from, until) = month().ist_bounds_micros();
    let mut out = vec![i64::MIN, from - 1, from, until - 1, until, i64::MAX];
    for pair in bars.windows(2) {
        let (a, b) = (pair[0].ts_micros, pair[1].ts_micros);
        out.extend([a - 1, a, a + 1, a + (b - a) / 2]);
    }
    if let Some(last) = bars.last() {
        out.extend([last.ts_micros - 1, last.ts_micros, last.ts_micros + 1]);
    }
    out
}

/// The names in the month's directory, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Every file under `dir`, recursively, as a sorted list of paths.
fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in fs::read_dir(&at).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                stack.push(entry.clone());
            }
            out.push(entry);
        }
    }
    out.sort();
    out
}

fn write_in_place(path: &Path, at: u64, bytes: &[u8]) {
    use std::os::unix::fs::FileExt as _;
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.write_all_at(bytes, at).unwrap();
}

#[test]
fn a_month_written_before_the_index_gets_the_one_appends_keep_and_nothing_else_moves() {
    // DB-064 / DB-G81: the 142 index one-second months from 2018-09 to 2024-07
    // were written before D-2329 and carry no `.tix`. The door builds the
    // index an appending writer would have kept — the same bytes — and the
    // bars, their checksums and the lock are exactly as they were.
    let tf = Timeframe::SECOND_1;
    let root = Root::new();
    let bars = sessions(3, 22_500, 1);
    write(&root, tf, &bars, 4_999);
    let kept_by_appends = fs::read(root.tix(tf)).unwrap();
    fs::remove_file(root.tix(tf)).unwrap();

    let dir = root.tix(tf).parent().unwrap().to_path_buf();
    let names_before = listing(&dir);
    let bin = fs::read(root.file(tf, FileKind::Bars)).unwrap();
    let crc = fs::read(root.file(tf, FileKind::Checksums)).unwrap();
    let bin_modified = fs::metadata(root.file(tf, FileKind::Bars))
        .unwrap()
        .modified()
        .unwrap();

    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
    drop(reader);

    assert_eq!(
        root.index(tf).unwrap(),
        IndexedMonth {
            n_valid: bars.len() as u64,
            outcome: Reindexed::Built(Why::Absent),
        }
    );
    assert_eq!(
        fs::read(root.tix(tf)).unwrap(),
        kept_by_appends,
        "the index an appending writer keeps, byte for byte"
    );
    assert_eq!(fs::read(root.file(tf, FileKind::Bars)).unwrap(), bin);
    assert_eq!(fs::read(root.file(tf, FileKind::Checksums)).unwrap(), crc);
    assert_eq!(
        fs::metadata(root.file(tf, FileKind::Bars))
            .unwrap()
            .modified()
            .unwrap(),
        bin_modified,
        "the bar file was never opened for writing"
    );
    let mut names_after = names_before.clone();
    names_after.push("2019-03.tix".to_owned());
    names_after.sort();
    assert_eq!(listing(&dir), names_after, "the .tix is the one new file");

    // A reader now goes through the index, and every answer is the
    // definition: the bars stamped before `ts`.
    let reader = root.reader(tf);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    for ts in probes(&bars) {
        let truth = bars.partition_point(|b| b.ts_micros < ts) as u64;
        assert_eq!(reader.first_at_or_after(ts), Ok(truth), "ts={ts}");
    }
    drop(reader);

    // A second run reads three things and writes nothing.
    let tix_modified = fs::metadata(root.tix(tf)).unwrap().modified().unwrap();
    assert_eq!(
        root.index(tf).unwrap(),
        IndexedMonth {
            n_valid: bars.len() as u64,
            outcome: Reindexed::Kept,
        }
    );
    assert_eq!(fs::read(root.tix(tf)).unwrap(), kept_by_appends);
    assert_eq!(
        fs::metadata(root.tix(tf)).unwrap().modified().unwrap(),
        tix_modified,
        "a kept index is not rewritten"
    );
}

#[test]
fn a_month_that_is_not_there_is_refused_and_nothing_is_created() {
    let tf = Timeframe::SECOND_1;
    let root = Root::new();
    assert_eq!(
        root.index(tf),
        Err(StoreError::Missing {
            path: root.file(tf, FileKind::Bars),
            action: store::file::Action::Open,
        })
    );
    assert!(tree(&root.0).is_empty(), "{:?}", tree(&root.0));

    // A neighbouring month exists; this one still is not made, and not even
    // a `.lock` appears beside it.
    let other = path_at(tf, YearMonth::new(2019, 4).unwrap(), FileKind::Bars);
    drop(BarFile::open_or_create(&root.0, other, SYMBOL).unwrap());
    let before = tree(&root.0);
    assert!(matches!(root.index(tf), Err(StoreError::Missing { .. })));
    assert_eq!(tree(&root.0), before);
}

#[test]
fn a_month_another_holder_has_is_refused_and_not_written() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    let bars = sessions(3, 375, 60);
    write(&root, tf, &bars, 500);
    fs::remove_file(root.tix(tf)).unwrap();
    let locked = Err(StoreError::Locked {
        path: root.file(tf, FileKind::Lock),
    });

    // A reader mid-request holds the month shared.
    let reader = root.reader(tf);
    assert_eq!(root.index(tf), locked);
    assert!(!root.tix(tf).exists());
    drop(reader);

    // A writer holds it exclusive. Its own open builds the index, which is
    // taken away again so the refusal is the only thing that can explain the
    // `.tix` staying absent.
    let writer = root.writer(tf);
    fs::remove_file(root.tix(tf)).unwrap();
    assert_eq!(root.index(tf), locked);
    assert!(!root.tix(tf).exists());
    drop(writer);

    // Released, the same call builds it.
    assert_eq!(
        root.index(tf).unwrap().outcome,
        Reindexed::Built(Why::Absent)
    );
    assert_eq!(root.reader(tf).time_lookup(), TimeLookup::Indexed);
}

#[test]
fn a_month_no_index_can_describe_is_named_and_left_without_one() {
    // D-2330: two daily bars on one IST day share a slot. A daily month is
    // under the small-month threshold, so with no `.tix` the door gives it
    // none and writes nothing (D-4933); with one left beside it — a month
    // written before the rule — the door says why no index can describe it,
    // leaves no `.tix` claiming the bars, and the month still answers by the
    // bisection.
    let tf = Timeframe::DAY_1;
    let root = Root::new();
    let mut writer = root.writer(tf);
    assert!(writer.append(&[bar(at(3, 0))]).is_ok());
    assert!(writer.append(&[bar(at(3, 3_600))]).is_ok());
    drop(writer);
    assert!(!root.tix(tf).exists(), "a daily month is given no index");
    let bin = fs::read(root.file(tf, FileKind::Bars)).unwrap();
    assert_eq!(
        root.index(tf).unwrap(),
        IndexedMonth {
            n_valid: 2,
            outcome: Reindexed::BelowThreshold,
        }
    );
    assert!(!root.tix(tf).exists(), "nothing written under the rule");
    let geometry = store::time_index::Geometry::new(month(), 86_400, SYMBOL);
    fs::write(root.tix(tf), geometry.encode()).unwrap();

    let shared = Why::SharedSlot {
        index: 1,
        ts_micros: at(3, 3_600),
    };
    assert_eq!(
        root.index(tf).unwrap(),
        IndexedMonth {
            n_valid: 2,
            outcome: Reindexed::Unindexable(shared),
        }
    );
    assert!(
        !root.tix(tf).exists(),
        "no index is left claiming the month"
    );
    assert_eq!(fs::read(root.file(tf, FileKind::Bars)).unwrap(), bin);
    let reader = root.reader(tf);
    assert_eq!(
        reader.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    assert_eq!(reader.first_at_or_after(at(3, 1)), Ok(1));
    assert_eq!(reader.first_at_or_after(at(3, 3_601)), Ok(2));
}

#[test]
fn a_stale_or_damaged_index_is_rebuilt_and_says_why() {
    let tf = Timeframe::MINUTE_1;
    let bars = sessions(3, 375, 60);
    let root = Root::new();
    // Past the small-month threshold, so the month carries an index to go
    // wrong (D-4933).
    write(&root, tf, &bars[..1_024], 1_024);
    let old = fs::read(root.tix(tf)).unwrap();
    write(&root, tf, &bars[1_024..], 1_000);
    let right = fs::read(root.tix(tf)).unwrap();

    // The month grew past an index that was then put back: the entry is
    // there and does not hold the last bar. Then a damaged header, and a file
    // shorter than one.
    let mut damaged = right.clone();
    damaged[20] ^= 1;
    // The old index has no entry for the bucket the last bar lies in.
    let geometry = store::time_index::Geometry::new(month(), 60, SYMBOL);
    let last_slot = geometry.slot_of(bars.last().unwrap().ts_micros).unwrap();
    let (last_bucket, _) = store::time_index::split(last_slot);
    for (bytes, why) in [
        (
            old,
            Why::Entry {
                bucket: last_bucket,
            },
        ),
        (damaged, Why::Header("its header fails its checksum")),
        (vec![0u8; 10], Why::Header("it is shorter than its header")),
    ] {
        fs::write(root.tix(tf), &bytes).unwrap();
        // The door names the same reason a reader bisects for.
        assert_eq!(
            root.reader(tf).time_lookup(),
            TimeLookup::Bisection(why.clone())
        );
        assert_eq!(
            root.index(tf).unwrap(),
            IndexedMonth {
                n_valid: bars.len() as u64,
                outcome: Reindexed::Built(why.clone()),
            }
        );
        assert_eq!(fs::read(root.tix(tf)).unwrap(), right, "{why:?}");
        assert_eq!(root.reader(tf).time_lookup(), TimeLookup::Indexed);
    }

    // Stale: the month grew by one bar inside the same bucket.
    let bucket = |bar: &Bar| store::time_index::split(geometry.slot_of(bar.ts_micros).unwrap()).0;
    let n = (1_024..bars.len())
        .find(|&n| bucket(&bars[n - 1]) == bucket(&bars[n]))
        .unwrap();
    let small = Root::new();
    write(&small, tf, &bars[..n], n);
    let old = fs::read(small.tix(tf)).unwrap();
    write(&small, tf, &bars[n..=n], 1);
    let right = fs::read(small.tix(tf)).unwrap();
    fs::write(small.tix(tf), &old).unwrap();
    assert_eq!(
        small.index(tf).unwrap().outcome,
        Reindexed::Built(Why::Stale)
    );
    assert_eq!(fs::read(small.tix(tf)).unwrap(), right);
}

#[test]
fn an_empty_month_keeps_or_gets_its_header_only_index() {
    let tf = Timeframe::SECOND_1;
    let root = Root::new();
    drop(root.writer(tf));
    let empty = |outcome| IndexedMonth {
        n_valid: 0,
        outcome,
    };
    // A NEW MONTH IS GIVEN NO INDEX (D-4933), and the door makes none.
    assert!(!root.tix(tf).exists());
    assert_eq!(root.index(tf).unwrap(), empty(Reindexed::BelowThreshold));
    assert!(!root.tix(tf).exists());

    // One left beside it by an earlier build is its header alone, and kept.
    let header_only = store::time_index::Geometry::new(month(), 1, SYMBOL)
        .encode()
        .to_vec();
    fs::write(root.tix(tf), &header_only).unwrap();
    assert_eq!(root.index(tf).unwrap(), empty(Reindexed::Kept));
    assert_eq!(fs::read(root.tix(tf)).unwrap(), header_only);

    // Entries past the header describe bars that never committed.
    write_in_place(&root.tix(tf), 64, &[0xAB; 32]);
    assert_eq!(fs::read(root.tix(tf)).unwrap().len(), 96);
    assert_eq!(root.index(tf).unwrap(), empty(Reindexed::Built(Why::Stale)));
    assert_eq!(fs::read(root.tix(tf)).unwrap(), header_only);

    // A header of the right length that does not check.
    write_in_place(&root.tix(tf), 20, &[header_only[20] ^ 1]);
    assert_eq!(
        root.index(tf).unwrap(),
        empty(Reindexed::Built(Why::Header(
            "its header fails its checksum"
        )))
    );
    assert_eq!(fs::read(root.tix(tf)).unwrap(), header_only);
}

#[test]
fn a_sidecar_path_a_link_below_the_root_and_a_wrong_symbol_are_refused_before_any_write() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    write(&root, tf, &sessions(3, 375, 60), 1_095);
    fs::remove_file(root.tix(tf)).unwrap();

    let overlay = path_at(tf, month(), FileKind::Overlay);
    assert_eq!(
        BarFile::index_existing(&root.0, overlay, SYMBOL),
        Err(StoreError::NotABarPath {
            found: FileKind::Overlay
        })
    );

    // A test server's shadow root links `bars` into the real store; the door
    // must never write through it.
    let shadow = Root::new();
    std::os::unix::fs::symlink(root.0.join("bars"), shadow.0.join("bars")).unwrap();
    assert_eq!(
        BarFile::index_existing(&shadow.0, path(tf), SYMBOL),
        Err(StoreError::Symlinked {
            path: shadow.0.join("bars"),
        })
    );
    assert!(!root.tix(tf).exists());

    // A header naming another symbol is refused by `validated`, before the
    // index step.
    assert!(matches!(
        BarFile::index_existing(&root.0, path(tf), SYMBOL + 1),
        Err(StoreError::SymbolMismatch {
            stored: SYMBOL,
            asked,
            ..
        }) if asked == SYMBOL + 1
    ));
    assert!(!root.tix(tf).exists());
}
