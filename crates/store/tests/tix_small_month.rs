//! The small-month rule through real files — D-4933, the store-compression
//! design's §7.1 (SC-0).
//!
//! A month that holds fewer than 1,024 bars is given no `.tix` time index and
//! no `.srt` sort-index run; the commit that brings it to 1,024 or more writes
//! both WHOLE; an index already beside a small month is kept, never
//! abandoned; and on both sides of the threshold every lookup answers what
//! the bars answer. Each case below fails on the writer before D-4933, which
//! gave every month both files from its first commit.
//!
//! Every bar is invented: stamps in 2031, at prices no instrument printed.

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
use store::file::Resorted;
use store::file::{Appended, BarFile, BatchFlush, DurableBatch, Reindexed, StoreError, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sort_index::{Shape, SortIndex};
use store::time_index::{INDEX_MIN_ROWS, Why, below_threshold};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SYMBOL: u32 = 31;
const MICROS: i64 = 1_000_000;
const THRESHOLD: u64 = 1_024;

/// A store root of its own, removed with it.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-small-{}-{}",
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

    fn srt(&self, timeframe: Timeframe) -> PathBuf {
        self.file(timeframe, FileKind::SortIndex)
    }

    fn writer(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_or_create(&self.0, path(timeframe), SYMBOL).unwrap()
    }

    fn reader(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_existing(&self.0, path(timeframe), SYMBOL).unwrap()
    }

    /// The `.srt` beside the month: `(runs, rows covered)`, or why not.
    fn sorted(&self, timeframe: Timeframe) -> Result<(usize, u64), String> {
        SortIndex::open(
            &self.srt(timeframe),
            &Shape::of(month(), timeframe.secs(), SYMBOL),
        )
        .map(|index| (index.runs().len(), index.coverage()))
        .map_err(|why| why.to_string())
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month() -> YearMonth {
    YearMonth::new(2031, 5).unwrap()
}

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: month(),
        file: FileKind::Bars,
    })
    .unwrap()
}

fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 66_666,
        high: 66_676,
        low: 66_656,
        close: 66_670,
        volume: 2,
        open_interest: OI_NULL,
    }
}

/// 09:15:00 IST on day `day` (0-based) of May 2031; day 4 is Monday 5 May.
fn session_open(day: i64) -> i64 {
    month().ist_bounds_micros().0 + (day * 86_400 + 555 * 60) * MICROS
}

/// `count` one-second bars from 09:15:00 IST on Monday 5 May 2031, every
/// 41st second left out so the month has gaps inside its session.
fn seconds(count: u64) -> Vec<Bar> {
    (0i64..)
        .filter(|k| k % 41 != 40)
        .take(usize::try_from(count).unwrap())
        .map(|k| bar(session_open(4) + k * MICROS))
        .collect()
}

/// Every probe worth asking of `bars`: each stamp, a microsecond either side,
/// the middle of every gap, and both ends of the month.
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

/// Asserts every probe answers the bars' own first-at-or-after.
fn answers_as_the_bars_do(reader: &BarFile, bars: &[Bar]) {
    for ts in probes(bars) {
        let want = u64::try_from(bars.partition_point(|bar| bar.ts_micros < ts)).unwrap();
        assert_eq!(reader.first_at_or_after(ts), Ok(want), "ts={ts}");
    }
}

fn committed(appended: Result<Appended, StoreError>) -> u64 {
    match appended {
        Ok(Appended::Committed { n_valid, .. }) => n_valid,
        other => panic!("not committed: {other:?}"),
    }
}

/// Writes `bars` in commits of the sizes `cuts` names, asserting after each
/// that the month has both indexes exactly when it holds the threshold.
fn write_in(root: &Root, timeframe: Timeframe, bars: &[Bar], cuts: &[usize]) {
    let mut writer = root.writer(timeframe);
    assert!(!root.tix(timeframe).exists(), "an empty month has no .tix");
    let mut at = 0;
    for &cut in cuts {
        let n = committed(writer.append(&bars[at..at + cut]));
        at += cut;
        let indexed = n >= THRESHOLD;
        assert_eq!(root.tix(timeframe).exists(), indexed, "after {n} bars");
        assert_eq!(root.srt(timeframe).exists(), indexed, "after {n} bars");
        let lookup = if indexed {
            TimeLookup::Indexed
        } else {
            TimeLookup::Bisection(Why::BelowThreshold)
        };
        assert_eq!(writer.time_lookup(), lookup, "the writer, after {n} bars");
    }
    assert_eq!(at, bars.len(), "the cuts cover the bars");
}

#[test]
fn the_threshold_is_1024_bars() {
    assert_eq!(INDEX_MIN_ROWS, THRESHOLD);
    assert!(below_threshold(0));
    assert!(below_threshold(1_023));
    assert!(!below_threshold(1_024));
    assert!(!below_threshold(u64::MAX));
}

#[test]
fn a_new_month_under_the_threshold_gets_no_tix_and_no_srt() {
    let root = Root::new();
    let bars = seconds(THRESHOLD - 1);
    write_in(&root, Timeframe::SECOND_1, &bars, &[1, 99, 400, 523]);
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(
        reader.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    answers_as_the_bars_do(&reader, &bars);
    assert!(!root.tix(Timeframe::SECOND_1).exists());
    assert!(!root.srt(Timeframe::SECOND_1).exists());
}

#[test]
fn the_boundaries_of_the_threshold_in_one_commit() {
    for n in [1, THRESHOLD - 1, THRESHOLD, THRESHOLD + 1] {
        let root = Root::new();
        let bars = seconds(n);
        write_in(&root, Timeframe::SECOND_1, &bars, &[bars.len()]);
        let reader = root.reader(Timeframe::SECOND_1);
        answers_as_the_bars_do(&reader, &bars);
        if n >= THRESHOLD {
            assert_eq!(reader.time_lookup(), TimeLookup::Indexed, "n={n}");
            assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, n)), "n={n}");
        } else {
            assert_eq!(
                reader.time_lookup(),
                TimeLookup::Bisection(Why::BelowThreshold),
                "n={n}"
            );
        }
    }
}

#[test]
fn the_commit_that_crosses_the_threshold_writes_both_whole() {
    // Landing on it, passing it, crossing from one bar, and crossing in the
    // middle of three commits.
    for cuts in [
        vec![1_023, 1],
        vec![1_000, 25],
        vec![1, 1_023],
        vec![512, 511, 2],
    ] {
        let root = Root::new();
        let n: usize = cuts.iter().sum();
        let bars = seconds(u64::try_from(n).unwrap());
        write_in(&root, Timeframe::SECOND_1, &bars, &cuts);
        let reader = root.reader(Timeframe::SECOND_1);
        assert_eq!(reader.time_lookup(), TimeLookup::Indexed, "{cuts:?}");
        answers_as_the_bars_do(&reader, &bars);
        // ONE RUN: the crossing commit found no `.srt` and wrote it whole.
        assert_eq!(
            root.sorted(Timeframe::SECOND_1),
            Ok((1, u64::try_from(n).unwrap())),
            "{cuts:?}"
        );
    }
}

#[test]
fn after_the_threshold_a_month_is_kept_in_step_as_any_other() {
    let root = Root::new();
    let bars = seconds(THRESHOLD + 300);
    write_in(&root, Timeframe::SECOND_1, &bars, &[1_023, 1, 1, 299]);
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars);
    // The whole run of the crossing, then one run per later commit.
    assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((3, THRESHOLD + 300)));
}

/// A RE-OFFER IS NO CROSSING, AND AN OVERLAP CROSSES BY ITS NEW BARS
/// (D-4933). A month of 1,000 bars offered those 1,000 again would count past
/// the threshold if the offer were added to what is held, but it holds no bar
/// more: the admission answers `AlreadyPresent` before the index step is
/// asked, and no `.tix` or `.srt` is made. An offer that overlaps the held
/// tail and runs on past the threshold commits only its new suffix, and that
/// commit writes both indexes whole.
#[test]
fn a_re_offer_is_no_crossing_and_an_overlap_crosses_by_its_new_bars() {
    let root = Root::new();
    let bars = seconds(1_100);
    let mut writer = root.writer(Timeframe::SECOND_1);
    assert_eq!(committed(writer.append(&bars[..1_000])), 1_000);
    assert!(matches!(
        writer.append(&bars[..1_000]),
        Ok(Appended::AlreadyPresent { .. })
    ));
    assert!(
        !root.tix(Timeframe::SECOND_1).exists(),
        "a re-offer writes no .tix"
    );
    assert!(
        !root.srt(Timeframe::SECOND_1).exists(),
        "a re-offer writes no .srt"
    );
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    assert_eq!(committed(writer.append(&bars[900..])), 1_100);
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    drop(writer);
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars);
    assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, 1_100)));
}

#[test]
fn a_writer_reopened_below_the_threshold_crosses_it_on_its_own_commit() {
    let root = Root::new();
    let bars = seconds(THRESHOLD + 10);
    {
        let mut first = root.writer(Timeframe::SECOND_1);
        assert_eq!(committed(first.append(&bars[..1_000])), 1_000);
    }
    let mut second = root.writer(Timeframe::SECOND_1);
    assert!(
        !root.tix(Timeframe::SECOND_1).exists(),
        "the reopen made none"
    );
    assert_eq!(committed(second.append(&bars[1_000..])), THRESHOLD + 10);
    drop(second);
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars);
    assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, THRESHOLD + 10)));
}

#[test]
fn a_minute_month_crosses_on_its_third_session() {
    let root = Root::new();
    let sessions: Vec<Vec<Bar>> = [4, 5, 6]
        .iter()
        .map(|&day| {
            (0..375)
                .map(|k| bar(session_open(day) + k * 60 * MICROS))
                .collect()
        })
        .collect();
    let all: Vec<Bar> = sessions.concat();
    write_in(&root, Timeframe::MINUTE_1, &all, &[375, 375, 375]);
    let reader = root.reader(Timeframe::MINUTE_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &all);
    assert_eq!(root.sorted(Timeframe::MINUTE_1), Ok((1, 1_125)));
}

#[test]
fn a_daily_month_never_reaches_it_and_two_bars_on_one_day_stay_silent() {
    let root = Root::new();
    // Weekdays of May 2031 at 09:15 IST, and a SECOND bar on the 5th, which
    // the daily rung admits (D-0915) and no `.tix` could hold.
    let mut bars: Vec<Bar> = (0..31)
        .filter(|day| ![2, 3, 9, 10, 16, 17, 23, 24, 30].contains(day))
        .map(|day| bar(session_open(day)))
        .collect();
    let second = bar(session_open(4) + 60 * MICROS);
    let at = bars.partition_point(|bar| bar.ts_micros <= session_open(4));
    bars.insert(at, second);
    write_in(&root, Timeframe::DAY_1, &bars, &[5, bars.len() - 5]);
    let reader = root.reader(Timeframe::DAY_1);
    assert_eq!(
        reader.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    answers_as_the_bars_do(&reader, &bars);
}

/// A copy of the `.tix` a month of `bars` would carry, made the only way a
/// small month can have one now: written beside a month of the same first
/// bars that DID reach the threshold. Its entries run ahead of the small
/// month's commit — the state every append's crash leaves.
fn index_written_ahead(timeframe: Timeframe, ahead: &[Bar]) -> Vec<u8> {
    let other = Root::new();
    write_in(&other, timeframe, ahead, &[ahead.len()]);
    fs::read(other.tix(timeframe)).unwrap()
}

#[test]
fn an_index_already_beside_a_small_month_is_kept_in_step_not_abandoned() {
    let root = Root::new();
    let bars = seconds(THRESHOLD + 6);
    let image = index_written_ahead(Timeframe::SECOND_1, &bars);
    write_in(&root, Timeframe::SECOND_1, &bars[..10], &[10]);
    fs::write(root.tix(Timeframe::SECOND_1), &image).unwrap();
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars[..10]);
    drop(reader);

    // A writer of the SMALL month keeps that index in step: were it left
    // alone it would stop describing the bars, and every reader would warn
    // that it is stale.
    {
        let mut writer = root.writer(Timeframe::SECOND_1);
        assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
        assert_eq!(committed(writer.append(&bars[10..15])), 15);
    }
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars[..15]);
    drop(reader);
    assert_eq!(
        BarFile::index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Reindexed::Kept
    );
}

#[test]
fn a_sort_index_already_beside_a_small_month_is_kept_correct() {
    let root = Root::new();
    let bars = seconds(THRESHOLD + 6);
    let other = Root::new();
    write_in(&other, Timeframe::SECOND_1, &bars, &[bars.len()]);
    write_in(&root, Timeframe::SECOND_1, &bars[..10], &[10]);
    // An `.srt` of OTHER bars beside the small month: it does not describe
    // these, so the writer's open writes the month whole rather than leave
    // it — as it always did for a damaged index, whatever the month holds —
    // and the next commit appends a run to it, as to any index.
    fs::copy(
        other.srt(Timeframe::SECOND_1),
        root.srt(Timeframe::SECOND_1),
    )
    .unwrap();
    {
        let mut writer = root.writer(Timeframe::SECOND_1);
        assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, 10)));
        assert_eq!(committed(writer.append(&bars[10..12])), 12);
    }
    assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((2, 12)));
    // The door, too, keeps it correct: two runs are written as one.
    assert!(matches!(
        BarFile::sort_index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Resorted::Built(_)
    ));
    assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, 12)));
}

#[test]
fn something_at_the_tix_name_of_a_small_month_is_refused_not_excused() {
    // A DIRECTORY at the name of a new month's `.tix`: the rule excuses only
    // a name that is not there, so the open refuses it as it always did.
    let root = Root::new();
    let tix = root.tix(Timeframe::SECOND_1);
    fs::create_dir_all(&tix).unwrap();
    assert!(BarFile::open_or_create(&root.0, path(Timeframe::SECOND_1), SYMBOL).is_err());

    // A LINK at the name of a small month that has bars: the reader names it
    // and does not call the month "below the threshold".
    let root = Root::new();
    let bars = seconds(10);
    write_in(&root, Timeframe::SECOND_1, &bars, &[10]);
    let elsewhere = root.0.join("elsewhere.tix");
    fs::write(&elsewhere, [0u8; 64]).unwrap();
    std::os::unix::fs::symlink(&elsewhere, root.tix(Timeframe::SECOND_1)).unwrap();
    let reader = root.reader(Timeframe::SECOND_1);
    assert!(
        matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
        "{:?}",
        reader.time_lookup()
    );
    answers_as_the_bars_do(&reader, &bars);
    drop(reader);
    assert!(BarFile::open_or_create(&root.0, path(Timeframe::SECOND_1), SYMBOL).is_err());
}

#[test]
fn a_torn_tix_beside_a_small_month_is_named_and_rebuilt() {
    let root = Root::new();
    let bars = seconds(10);
    write_in(&root, Timeframe::SECOND_1, &bars, &[10]);
    // A crash between creating the threshold's `.tix` and writing its header
    // leaves a short file; the rule does not excuse it.
    fs::write(root.tix(Timeframe::SECOND_1), [0u8; 10]).unwrap();
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(
        reader.time_lookup(),
        TimeLookup::Bisection(Why::Header("it is shorter than its header"))
    );
    answers_as_the_bars_do(&reader, &bars);
    drop(reader);
    assert!(matches!(
        BarFile::index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Reindexed::Built(Why::Header(_))
    ));
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    answers_as_the_bars_do(&reader, &bars);
}

#[test]
fn a_month_at_the_threshold_without_its_tix_is_still_a_finding() {
    let root = Root::new();
    let bars = seconds(THRESHOLD);
    write_in(&root, Timeframe::SECOND_1, &bars, &[bars.len()]);
    fs::remove_file(root.tix(Timeframe::SECOND_1)).unwrap();
    let reader = root.reader(Timeframe::SECOND_1);
    assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
    answers_as_the_bars_do(&reader, &bars);
    drop(reader);
    assert_eq!(
        BarFile::index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Reindexed::Built(Why::Absent)
    );
    assert_eq!(
        root.reader(Timeframe::SECOND_1).time_lookup(),
        TimeLookup::Indexed
    );
}

#[test]
fn the_rebuild_doors_know_the_rule_and_write_nothing_under_it() {
    let root = Root::new();
    let bars = seconds(THRESHOLD - 1);
    write_in(&root, Timeframe::SECOND_1, &bars, &[bars.len()]);
    let indexed = BarFile::index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    assert_eq!(
        (indexed.n_valid, indexed.outcome),
        (THRESHOLD - 1, Reindexed::BelowThreshold)
    );
    let sorted = BarFile::sort_index_existing(&root.0, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    assert_eq!(
        (sorted.n_valid, sorted.runs, sorted.outcome),
        (THRESHOLD - 1, 0, Resorted::BelowThreshold)
    );
    assert!(!root.tix(Timeframe::SECOND_1).exists());
    assert!(!root.srt(Timeframe::SECOND_1).exists());

    // An EMPTY month is still `Empty` to the sort door, and below the rule
    // to the time door; neither writes a file.
    let empty = Root::new();
    drop(empty.writer(Timeframe::SECOND_1));
    assert_eq!(
        BarFile::index_existing(&empty.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Reindexed::BelowThreshold
    );
    assert_eq!(
        BarFile::sort_index_existing(&empty.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Resorted::Empty
    );
    assert!(!empty.tix(Timeframe::SECOND_1).exists());
    assert!(!empty.srt(Timeframe::SECOND_1).exists());

    // At the threshold, a month that lost its `.srt` is built again.
    let full = Root::new();
    let bars = seconds(THRESHOLD);
    write_in(&full, Timeframe::SECOND_1, &bars, &[bars.len()]);
    fs::remove_file(full.srt(Timeframe::SECOND_1)).unwrap();
    assert!(matches!(
        BarFile::sort_index_existing(&full.0, path(Timeframe::SECOND_1), SYMBOL)
            .unwrap()
            .outcome,
        Resorted::Built(_)
    ));
    assert_eq!(full.sorted(Timeframe::SECOND_1), Ok((1, THRESHOLD)));
}

#[test]
fn a_durable_batch_crosses_the_threshold_as_a_plain_writer_does() {
    for flush in [BatchFlush::OncePerDevice, BatchFlush::EveryFile] {
        let root = Root::new();
        let bars = seconds(THRESHOLD + 76);
        let batch = DurableBatch::new(flush);
        {
            let _inside = batch.enter();
            write_in(&root, Timeframe::SECOND_1, &bars, &[1_000, 100]);
        }
        batch.make_durable(1).unwrap();
        let reader = root.reader(Timeframe::SECOND_1);
        assert_eq!(reader.time_lookup(), TimeLookup::Indexed, "{flush:?}");
        answers_as_the_bars_do(&reader, &bars);
        assert_eq!(root.sorted(Timeframe::SECOND_1), Ok((1, THRESHOLD + 76)));
    }
}

#[test]
fn a_bar_the_crossing_commit_cannot_hold_is_refused_before_any_byte() {
    let root = Root::new();
    let bars = seconds(THRESHOLD - 1);
    let mut writer = root.writer(Timeframe::SECOND_1);
    assert_eq!(committed(writer.append(&bars)), THRESHOLD - 1);
    // Off the one-second grid, in the commit that would cross.
    let last = bars[bars.len() - 1].ts_micros;
    let crossing = [bar(last + MICROS), bar(last + 2 * MICROS + 500)];
    assert!(writer.append(&crossing).is_err());
    assert_eq!(writer.header().n_valid, THRESHOLD - 1);
    assert!(!root.tix(Timeframe::SECOND_1).exists());
    // And the month crosses cleanly on the next good commit.
    assert_eq!(committed(writer.append(&crossing[..1])), THRESHOLD);
    assert!(root.tix(Timeframe::SECOND_1).exists());
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
}
