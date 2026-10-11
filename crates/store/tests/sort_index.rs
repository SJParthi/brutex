//! The `.srt` sort index a writer keeps, and the door `cli srt-build`
//! walks. D-3349, WP-39; DB-N14.
//!
//! What each test holds: every commit is followed by one run over exactly the
//! rows it committed, and a reader's entries are a brute-force sort of the
//! bars; a month written before D-3349 is ordered whole at its first writer
//! open; a run lost to a crash is appended at the next open; a month past
//! `COMPACT_AT` runs is written as one; the bar file, its checksums and its
//! time index never change; the rebuild door writes one run as a new file,
//! keeps it on a rerun, and refuses a month that is held or not there; a run
//! torn by a crash is walked past and appended again; and a run that cannot
//! be written never refuses the bars it would have ordered.
//!
//! Every bar is invented: one-second stamps in 2031-02 at prices no index
//! ever printed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, Resorted, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sort_index::{COMPACT_AT, Column, Entry, RunKind, Shape, SortIndex, Why};

static NEXT: AtomicU64 = AtomicU64::new(0);

const SYMBOL: u32 = 11;
const MICROS: i64 = 1_000_000;

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-srt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, kind: FileKind) -> PathBuf {
        path(kind).to_path_buf(&self.0)
    }

    fn writer(&self) -> BarFile {
        BarFile::open_or_create(&self.0, path(FileKind::Bars), SYMBOL).unwrap()
    }

    fn index(&self) -> Result<SortIndex, Why> {
        SortIndex::open(&self.file(FileKind::SortIndex), &shape())
    }

    fn digest(&self, kind: FileKind) -> Vec<u8> {
        fs::read(self.file(kind)).unwrap_or_default()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month() -> YearMonth {
    YearMonth::new(2031, 2).unwrap()
}

fn shape() -> Shape {
    Shape::of(month(), 1, SYMBOL)
}

fn path(file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: month(),
        file,
    })
    .unwrap()
}

/// 09:15 IST on day `day` of the month, plus `secs`.
fn at(day: i64, secs: i64) -> i64 {
    month().ist_bounds_micros().0 + day * 86_400 * MICROS + (555 * 60 + secs) * MICROS
}

/// Bar `i` of an invented series: closes that wander and repeat, volumes
/// that tie often, an open interest that is sometimes null.
fn bar(i: i64) -> Bar {
    let close = 7_770_000 + (i * 7919) % 613;
    Bar {
        ts_micros: at(i / 20_000, i % 20_000),
        open: close - 11,
        high: close + 40,
        low: close - 40,
        close,
        volume: (i * 31) % 19,
        open_interest: if i % 5 == 0 { OI_NULL } else { 900 + i % 7 },
    }
}

fn bars(from: i64, to: i64) -> Vec<Bar> {
    (from..to).map(bar).collect()
}

fn append(writer: &mut BarFile, rows: &[Bar]) {
    assert!(matches!(
        writer.append(rows),
        Ok(Appended::Committed { .. })
    ));
}

/// Every entry of `column`, across every run, as the reader serves them.
fn served(index: &SortIndex, column: Column) -> Vec<Vec<Entry>> {
    (0..index.runs().len())
        .map(|run| {
            let fences = index.fences(run, column).unwrap();
            (0..index.runs()[run].column(column).blocks)
                .flat_map(|block| index.block(run, column, block, &fences).unwrap())
                .collect()
        })
        .collect()
}

/// The brute-force order of rows `[from, to)` by `column`: nulls first in
/// time order, then `(value, time)`.
fn brute(from: i64, to: i64, column: Column) -> Vec<Entry> {
    let origin = shape().origin_micros;
    let mut all: Vec<(bool, Entry)> = bars(from, to)
        .iter()
        .map(|b| {
            let v = column.of(b);
            let null = column.nullable() && v == OI_NULL;
            (
                !null,
                Entry {
                    v,
                    sec: u32::try_from((b.ts_micros - origin) / MICROS).unwrap(),
                },
            )
        })
        .collect();
    all.sort_by_key(|(valued, e)| (*valued, if *valued { e.v } else { 0 }, e.sec));
    all.into_iter().map(|(_, e)| e).collect()
}

/// **EVERY COMMIT IS FOLLOWED BY ONE RUN OVER EXACTLY ITS ROWS**, and what a
/// reader serves is a brute-force sort of those rows.
#[test]
fn each_commit_appends_one_run_over_exactly_its_rows() {
    let root = Root::new();
    let mut writer = root.writer();
    assert!(
        !root.file(FileKind::SortIndex).exists(),
        "an empty month orders nothing and writes nothing"
    );
    append(&mut writer, &bars(0, 1_000));
    append(&mut writer, &bars(1_000, 1_001));
    append(&mut writer, &bars(1_001, 2_600));
    let index = root.index().unwrap();
    let spans: Vec<(u64, u64)> = index.runs().iter().map(|r| (r.start, r.end)).collect();
    assert_eq!(spans, vec![(0, 1_000), (1_000, 1_001), (1_001, 2_600)]);
    assert_eq!(index.coverage(), 2_600);
    let kinds: Vec<RunKind> = index.runs().iter().map(|run| run.kind).collect();
    assert_eq!(
        kinds,
        [RunKind::Whole, RunKind::Appended, RunKind::Appended],
        "the first commit finds no index and writes the month whole; each later one appends"
    );
    for column in Column::ALL {
        let got = served(&index, column);
        assert_eq!(got[0], brute(0, 1_000, column), "{}", column.as_str());
        assert_eq!(got[1], brute(1_000, 1_001, column), "{}", column.as_str());
        assert_eq!(got[2], brute(1_001, 2_600, column), "{}", column.as_str());
    }
    // A re-run of the same batch is already present and adds no run.
    assert!(matches!(
        writer.append(&bars(1_001, 2_600)),
        Ok(Appended::AlreadyPresent { .. })
    ));
    assert_eq!(root.index().unwrap().runs().len(), 3);
}

/// A month written before D-3349 has no `.srt`; its first writer open orders
/// it whole, as one run, and changes no byte of the bars, the checksums or
/// the time index.
#[test]
fn a_writer_open_orders_a_month_that_predates_the_index_whole() {
    let root = Root::new();
    {
        let mut writer = root.writer();
        append(&mut writer, &bars(0, 700));
        append(&mut writer, &bars(700, 1_500));
    }
    fs::remove_file(root.file(FileKind::SortIndex)).unwrap();
    let before = [
        root.digest(FileKind::Bars),
        root.digest(FileKind::Checksums),
        root.digest(FileKind::TimeIndex),
    ];
    drop(root.writer());
    let after = [
        root.digest(FileKind::Bars),
        root.digest(FileKind::Checksums),
        root.digest(FileKind::TimeIndex),
    ];
    assert_eq!(before, after);
    let index = root.index().unwrap();
    assert_eq!(index.runs().len(), 1);
    assert_eq!((index.runs()[0].start, index.runs()[0].end), (0, 1_500));
    assert_eq!(
        index.runs()[0].kind,
        RunKind::Whole,
        "written whole, by rename"
    );
    assert_eq!(
        served(&index, Column::Close)[0],
        brute(0, 1_500, Column::Close)
    );
    assert!(
        !root
            .file(FileKind::Bars)
            .with_file_name("2031-02.new.srt")
            .exists(),
        "the staging file is renamed, not left"
    );
}

/// A run lost to a crash between the commit and the run — here, cut off the
/// file — is appended at the next writer open, over exactly the rows the
/// index no longer covers.
#[test]
fn a_run_lost_after_its_commit_is_appended_at_the_next_open() {
    let root = Root::new();
    {
        let mut writer = root.writer();
        append(&mut writer, &bars(0, 400));
        append(&mut writer, &bars(400, 900));
    }
    let index = root.index().unwrap();
    let cut = index.runs()[1].offset;
    drop(index);
    let srt = fs::OpenOptions::new()
        .write(true)
        .open(root.file(FileKind::SortIndex))
        .unwrap();
    srt.set_len(cut + 100).unwrap();
    drop(srt);
    let torn = root.index().unwrap();
    assert_eq!(torn.coverage(), 400, "the torn run is not read as a run");
    assert!(torn.torn().is_some());
    drop(torn);
    drop(root.writer());
    let index = root.index().unwrap();
    let spans: Vec<(u64, u64)> = index.runs().iter().map(|r| (r.start, r.end)).collect();
    assert_eq!(spans, vec![(0, 400), (400, 900)]);
    assert!(index.torn().is_none(), "the torn bytes were cut, not kept");
    assert_eq!(
        served(&index, Column::Volume)[1],
        brute(400, 900, Column::Volume)
    );
}

/// However often a month is appended to, a reader never walks
/// `COMPACT_AT` runs: the writer writes the month whole first.
#[test]
fn a_month_appended_past_the_compaction_bound_is_written_as_one_run() {
    let root = Root::new();
    let mut writer = root.writer();
    let appends = i64::try_from(COMPACT_AT).unwrap() + 8;
    for k in 0..appends {
        append(&mut writer, &bars(k * 10, k * 10 + 10));
        let runs = root.index().unwrap().runs().len();
        assert!(runs < COMPACT_AT, "{runs} runs after append {k}");
    }
    let index = root.index().unwrap();
    assert_eq!(index.coverage(), u64::try_from(appends * 10).unwrap());
    assert_eq!(index.runs().len(), 9);
    assert_eq!(index.runs()[0].start, 0);
    assert_eq!(
        index.runs()[0].end,
        u64::try_from(i64::try_from(COMPACT_AT).unwrap() * 10).unwrap()
    );
    let all = served(&index, Column::OpenInterest);
    assert_eq!(
        all[0],
        brute(
            0,
            i64::try_from(COMPACT_AT).unwrap() * 10,
            Column::OpenInterest
        )
    );
}

/// An index that does not describe the bars — a file for another symbol
/// copied in — is written whole at the next writer open.
#[test]
fn an_index_that_is_not_this_months_is_written_whole() {
    let root = Root::new();
    {
        let mut writer = root.writer();
        append(&mut writer, &bars(0, 300));
    }
    let mut foreign = Shape::of(month(), 1, SYMBOL + 1).encode().to_vec();
    foreign.extend_from_slice(&[0u8; 40]);
    fs::write(root.file(FileKind::SortIndex), &foreign).unwrap();
    assert!(matches!(root.index(), Err(Why::Header(_))));
    drop(root.writer());
    let index = root.index().unwrap();
    assert_eq!(index.runs().len(), 1);
    assert_eq!(index.coverage(), 300);
}

/// **A RUN THAT CANNOT BE WRITTEN NEVER REFUSES THE BARS.** The `.srt` name
/// is a directory, so no run can land; the append still commits, and the
/// reader is refused by name rather than served a wrong order.
#[test]
fn a_run_that_cannot_be_written_never_refuses_the_commit() {
    let root = Root::new();
    let mut writer = root.writer();
    fs::create_dir_all(root.file(FileKind::SortIndex)).unwrap();
    append(&mut writer, &bars(0, 50));
    assert_eq!(writer.header().n_valid, 50);
    assert!(matches!(root.index(), Err(Why::Unreadable(_))));
}

/// The rebuild door: absent → written whole as one run in a new file;
/// rerun → kept with no byte changed; several runs → written as one; the bar
/// file, checksums and time index untouched throughout.
#[test]
fn the_rebuild_door_writes_one_run_and_keeps_it_on_a_rerun() {
    let root = Root::new();
    {
        let mut writer = root.writer();
        append(&mut writer, &bars(0, 500));
        append(&mut writer, &bars(500, 1_200));
    }
    let bars_before = root.digest(FileKind::Bars);
    let crc_before = root.digest(FileKind::Checksums);
    let tix_before = root.digest(FileKind::TimeIndex);

    let two = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(two.n_valid, 1_200);
    assert_eq!(two.runs, 1);
    assert!(
        matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
        "{:?}",
        two.outcome
    );
    let written = root.digest(FileKind::SortIndex);
    assert_eq!(root.index().unwrap().runs()[0].kind, RunKind::Whole);
    let again = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(again.outcome, Resorted::Kept);
    assert_eq!(
        root.digest(FileKind::SortIndex),
        written,
        "a rerun changes no byte"
    );

    fs::remove_file(root.file(FileKind::SortIndex)).unwrap();
    let absent = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
    assert_eq!(
        root.digest(FileKind::SortIndex),
        written,
        "the same bytes, rebuilt"
    );

    assert_eq!(root.digest(FileKind::Bars), bars_before);
    assert_eq!(root.digest(FileKind::Checksums), crc_before);
    assert_eq!(root.digest(FileKind::TimeIndex), tix_before);
    let index = root.index().unwrap();
    assert_eq!(
        served(&index, Column::High)[0],
        brute(0, 1_200, Column::High)
    );
}

/// **A CRASH THAT TORE THE LAST RUN COSTS NOTHING BUT A RE-APPEND.** The
/// last run's header landed and its body did not (zeros where entries were):
/// a reader walks past it — the rows it claimed are not covered, so they are
/// read — and the next writer open appends a run over exactly them, after
/// which every column reads back as the brute force again.
#[test]
fn a_torn_last_run_is_walked_past_and_appended_again() {
    let root = Root::new();
    {
        let mut writer = root.writer();
        append(&mut writer, &bars(0, 900));
        append(&mut writer, &bars(900, 1_700));
    }
    let srt = root.file(FileKind::SortIndex);
    let len = fs::metadata(&srt).unwrap().len();
    let file = fs::OpenOptions::new().write(true).open(&srt).unwrap();
    std::os::unix::fs::FileExt::write_all_at(&file, &[0u8; 512], len - 512).unwrap();
    drop(file);
    let index = root.index().unwrap();
    assert_eq!(index.coverage(), 900, "the torn run is not read");
    assert!(
        index
            .torn()
            .is_some_and(|(_, why)| why.contains("body fails its checksum")),
        "{:?}",
        index.torn()
    );
    drop(root.writer());
    let index = root.index().unwrap();
    let spans: Vec<(u64, u64)> = index.runs().iter().map(|r| (r.start, r.end)).collect();
    assert_eq!(spans, vec![(0, 900), (900, 1_700)]);
    assert!(index.torn().is_none());
    for column in Column::ALL {
        assert_eq!(served(&index, column)[1], brute(900, 1_700, column));
    }
}

/// The door refuses a month a writer holds, and a month that is not there,
/// and creates nothing for either.
#[test]
fn the_rebuild_door_refuses_a_held_or_absent_month_and_creates_nothing() {
    let root = Root::new();
    let missing = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL);
    assert!(
        matches!(missing, Err(StoreError::Missing { .. })),
        "{missing:?}"
    );
    assert_eq!(
        fs::read_dir(&root.0).unwrap().count(),
        0,
        "nothing was created"
    );

    let mut writer = root.writer();
    append(&mut writer, &bars(0, 10));
    let before = root.digest(FileKind::SortIndex);
    let held = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL);
    assert!(matches!(held, Err(StoreError::Locked { .. })), "{held:?}");
    assert_eq!(root.digest(FileKind::SortIndex), before);
    drop(writer);
    assert!(matches!(
        BarFile::sort_index_existing(&root.0, path(FileKind::SortIndex), SYMBOL),
        Err(StoreError::NotABarPath { .. })
    ));
}

/// **THE DOOR FOLLOWS NO LINK BELOW THE ROOT.** A store root whose month is
/// reached through a link — the rung directory, the symbol's, or `bars`
/// itself linked to another store, as a read-only shadow of a real store is
/// built — is refused by name at every depth, and nothing is written on
/// either side of the link: no `.srt`, no staging file, no byte of the bars,
/// their checksums or their time index. A shadow can therefore never build
/// an index into the store it shadows.
#[test]
fn the_rebuild_door_follows_no_link_below_the_root() {
    let real = Root::new();
    {
        let mut writer = real.writer();
        append(&mut writer, &bars(0, 400));
    }
    fs::remove_file(real.file(FileKind::SortIndex)).unwrap();
    let before: Vec<(String, Vec<u8>)> = {
        let dir = real.file(FileKind::Bars);
        let mut held: Vec<(String, Vec<u8>)> = fs::read_dir(dir.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap())
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        held.sort();
        held
    };
    // `bars/gdfl/NSE/INDEX/NIFTY/1s`: each depth below the root, linked in
    // turn into the real store.
    let below = ["bars", "gdfl", "NSE", "INDEX", "NIFTY", "1s"];
    for depth in 1..=below.len() {
        let shadow = Root::new();
        let mut at = shadow.0.clone();
        let mut target = real.0.clone();
        for part in &below[..depth - 1] {
            at.push(part);
            target.push(part);
        }
        fs::create_dir_all(&at).unwrap();
        std::os::unix::fs::symlink(target.join(below[depth - 1]), at.join(below[depth - 1]))
            .unwrap();
        assert!(
            shadow.file(FileKind::Bars).exists(),
            "depth {depth}: the month is readable through the link"
        );
        let refused = BarFile::sort_index_existing(&shadow.0, path(FileKind::Bars), SYMBOL);
        assert!(
            matches!(refused, Err(StoreError::Symlinked { .. })),
            "depth {depth}: {refused:?}"
        );
    }
    let after: Vec<(String, Vec<u8>)> = {
        let dir = real.file(FileKind::Bars);
        let mut held: Vec<(String, Vec<u8>)> = fs::read_dir(dir.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap())
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        held.sort();
        held
    };
    assert_eq!(
        before, after,
        "no file was added to, removed from or changed in the linked store"
    );
    assert!(!real.file(FileKind::SortIndex).exists());
}

/// **THE SERVED GENERATION IS THE ONE SORTED** (D-3349 under D-3350). A
/// versioned repair publishes the month again under `bar-revisions-v1/<n>/`
/// through an ordinary writer, so the revision is born with its own `.srt`
/// over its own rows. The rebuild door keeps it where it lives, rebuilds it
/// there when it is gone, and never touches the original month's index, which
/// no reader opens once the revision is promoted. Before, the door sorted the
/// ORIGINAL `.bin` and reported the month indexed while every reader read the
/// revision unindexed.
#[test]
fn the_rebuild_door_sorts_the_served_generation() {
    let root = Root::new();
    // THE DB-N20 SHAPE: the month holds day 17 on and lacks days 3 and 4.
    let on = |day: i64, i: i64| Bar {
        ts_micros: at(day, i),
        open_interest: OI_NULL,
        ..bar(day * 100 + i)
    };
    let held: Vec<Bar> = (0..60).map(|i| on(17, i)).collect();
    let leading: Vec<Bar> = [3, 4]
        .iter()
        .flat_map(|&day| (0..40).map(move |i| on(day, i)))
        .collect();
    {
        let mut writer = root.writer();
        append(&mut writer, &held);
    }
    let original_srt = root.digest(FileKind::SortIndex);
    assert!(!original_srt.is_empty(), "the original was kept indexed");
    let offered: Vec<Bar> = leading.iter().chain(&held).copied().collect();
    store::repair::repair_month(
        &root.0,
        path(FileKind::Bars),
        SYMBOL,
        &offered,
        store::repair::Policy {
            open_interest_applies: false,
            basis_witness: false,
        },
    )
    .unwrap();
    let revision_srt = path(FileKind::SortIndex).to_path_buf(&root.0.join("bar-revisions-v1/1"));
    assert!(revision_srt.is_file(), "the revision is born sorted");
    let shape = Shape::of(month(), 1, SYMBOL);
    let born = SortIndex::open(&revision_srt, &shape).unwrap();
    assert_eq!(born.coverage(), 140, "every row of the served generation");

    let kept = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(kept.n_valid, 140, "the served generation's rows");
    // One run, as a single commit of the revision leaves it.
    assert_eq!(kept.outcome, Resorted::Kept);

    fs::remove_file(&revision_srt).unwrap();
    let built = BarFile::sort_index_existing(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(built.n_valid, 140);
    assert!(
        matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
        "{:?}",
        built.outcome
    );
    let index = SortIndex::open(&revision_srt, &shape).unwrap();
    assert_eq!((index.runs().len(), index.coverage()), (1, 140));
    let fences = index.fences(0, Column::Close).unwrap();
    let closes: Vec<Entry> = (0..index.runs()[0].column(Column::Close).blocks)
        .flat_map(|block| index.block(0, Column::Close, block, &fences).unwrap())
        .collect();
    let mut want: Vec<Entry> = offered
        .iter()
        .map(|b| Entry {
            v: b.close,
            sec: u32::try_from((b.ts_micros - shape.origin_micros) / MICROS).unwrap(),
        })
        .collect();
    want.sort_by_key(|e| (e.v, e.sec));
    assert_eq!(closes, want, "a brute-force sort of the revision's rows");
    assert_eq!(
        root.digest(FileKind::SortIndex),
        original_srt,
        "never the original's index"
    );
}
