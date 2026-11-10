//! Extreme cases of ONE month of the store, written for the cloud extremes
//! round of 10 Nov 2026 (`EXTREMES-REPORT.md` at the repository root).
//!
//! The cases the rest of `crates/store/tests` already pins are not repeated
//! here. What is added is the edge each of them stops short of:
//!
//! * the reader-side lookups (`first_at_or_after`, the cold read) on a month
//!   holding nothing and holding one bar;
//! * a month holding a bar in EVERY slot its rung admits (the leap February,
//!   41,760 one-minute bars), written in one batch and in 29, and the one bar
//!   more that it must refuse;
//! * a batch that carries one stamp twice, with the copies agreeing and
//!   disagreeing;
//! * late bars interleaved with held ones, and a re-offered superset whose
//!   overlap is right or one paisa wrong;
//! * a zero volume, a zero open interest, a price of zero, and the largest
//!   values an `i64` holds;
//! * 29 February, a February with no 29th (2023, 2100), and the IST year
//!   boundary;
//! * the same input written twice, and into two fresh roots.
//!
//! Every value is invented. A test named `finding_*` is `#[ignore]`d because
//! it FAILS on the branch it was written against; its doc comment is the
//! finding (input, expected, actual, likely cause, proposed fix).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, HEADER_LEN, OI_NULL, RECORD_STRIDE};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Scratch and fixtures
// ===========================================================================

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A fresh directory under the host's temp dir, removed on drop.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("brutex-xm-{}-{tag}-{serial}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

/// The symbol id every month here is opened with.
const SYMBOL: u32 = 26_000;
/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;
/// One day, in microseconds.
const DAY: i64 = 86_400_000_000;

/// Days since 1970-01-01 of a proleptic Gregorian date. Written here rather
/// than borrowed, so the month bounds the store computes are checked against
/// an independent spelling of the calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The epoch microseconds of an IST wall-clock minute.
fn ist(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 - 19_800;
    secs * 1_000_000
}

/// A sane bar at `ts`, its prices moved by `seed` so two bars differ.
fn bar(ts: i64, seed: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open: 2_000_000 + seed,
        high: 2_000_500 + seed,
        low: 1_999_500 + seed,
        close: 2_000_100 + seed,
        volume: 10 + seed,
        open_interest: OI_NULL,
    }
}

/// `count` consecutive one-minute bars from `from`, seeded by their minute.
fn minutes(from: i64, count: i64) -> Vec<Bar> {
    (0..count).map(|i| bar(from + i * MINUTE, i)).collect()
}

/// The 1-minute NIFTY bar month `year-month` under a GDFL root.
fn month(year: u16, month: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(year, month).expect("a legal month"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn create(root: &Path, path: StorePath<'_>) -> BarFile {
    BarFile::open_or_create(root, path, SYMBOL).expect("the month opens for writing")
}

/// Every file beside the month's bar file that belongs to this month, by
/// name, with its bytes; the lock file is left out (it holds no data).
fn month_files(root: &Path, path: StorePath<'_>) -> BTreeMap<String, Vec<u8>> {
    let bars = path.to_path_buf(root);
    let dir = bars.parent().expect("a month directory");
    let stem = bars
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("a month stem")
        .to_owned();
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).expect("the month directory") {
        let entry = entry.expect("an entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let ours = name.starts_with(&format!("{stem}."));
        let lock = std::path::Path::new(&name)
            .extension()
            .is_some_and(|e| e == "lock");
        if ours && !lock && entry.file_type().expect("a type").is_file() {
            out.insert(name, fs::read(entry.path()).expect("the file"));
        }
    }
    out
}

/// The bar file's bytes.
fn image(root: &Path, path: StorePath<'_>) -> Vec<u8> {
    fs::read(path.to_path_buf(root)).expect("the bar file")
}

/// Every committed record of an open month, in index order.
fn records(file: &BarFile) -> Vec<Bar> {
    (0..file.records())
        .map(|i| file.read_record(i).expect("a committed record reads"))
        .collect()
}

// ===========================================================================
// Empty and one bar: the reader-side lookups at n = 0 and n = 1
// ===========================================================================

/// A month that holds nothing answers every lookup as empty, through the
/// writer's handle, the reader door and the audited door alike.
#[test]
fn an_empty_month_answers_every_lookup_as_empty_through_every_door() {
    let s = Scratch::new("empty");
    let path = month(2024, 6);
    let written = create(s.root(), path);
    assert_eq!(written.records(), 0);
    assert_eq!(written.first_at_or_after(i64::MIN), Ok(0));
    assert_eq!(written.first_at_or_after(i64::MAX), Ok(0));
    drop(written);

    let read = BarFile::open_existing(s.root(), path, SYMBOL).expect("the reader door");
    assert_eq!(read.records(), 0);
    assert!(
        matches!(read.read_record(0), Err(StoreError::NotCommitted { .. })),
        "index 0 of an empty month is not a record"
    );
    assert_eq!(read.first_at_or_after(ist(2024, 6, 3, 9, 15)), Ok(0));
    drop(read);

    let audited = audit(s.root(), path);
    assert!(audited.is_ok(), "an empty month audits clean: {audited:?}");
    assert_eq!(
        u64::try_from(image(s.root(), path).len()).unwrap(),
        HEADER_LEN,
        "nothing past the header region"
    );
}

/// One bar: every lookup around it lands on 0 or 1, never past the counter,
/// through the writer and after a reopen.
#[test]
fn a_one_bar_month_answers_every_lookup_at_its_edges() {
    let s = Scratch::new("one");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let one = bar(t, 0);
    let mut file = create(s.root(), path);
    assert_eq!(
        file.append(&[one]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 1
        })
    );
    assert_eq!(file.first_at_or_after(t - MINUTE), Ok(0));
    assert_eq!(file.first_at_or_after(t), Ok(0));
    assert_eq!(file.first_at_or_after(t + 1), Ok(1));
    assert_eq!(file.first_at_or_after(t + MINUTE), Ok(1));
    drop(file);
    let read = BarFile::open_existing(s.root(), path, SYMBOL).expect("the reader door");
    assert_eq!(records(&read), vec![one]);
    assert_eq!(read.header().first_ts_micros, t);
    assert_eq!(read.header().last_ts_micros, t);
    assert_eq!(read.first_at_or_after(t), Ok(0));
    assert_eq!(read.first_at_or_after(t + MINUTE), Ok(1));
    drop(read);
    let audited = audit(s.root(), path);
    assert!(audited.is_ok(), "a one-bar month audits clean: {audited:?}");
}

// ===========================================================================
// Very large: every slot of the month
// ===========================================================================

/// The bars of every minute of February 2024, 00:00 IST on the 1st to 23:59
/// IST on the 29th: 29 x 1,440 = 41,760.
fn every_minute_of_leap_february() -> Vec<Bar> {
    let from = ist(2024, 2, 1, 0, 0);
    minutes(from, 29 * 1_440)
}

/// A month holding a bar in every slot of its rung is full: all 41,760 bars
/// read back exactly at their computed offsets, the next instant is refused by
/// name, and the refusal leaves every byte where it was.
#[test]
fn a_month_with_a_bar_in_every_minute_slot_reads_back_whole_and_refuses_one_more() {
    let s = Scratch::new("full");
    let path = month(2024, 2);
    let all = every_minute_of_leap_february();
    assert_eq!(all.len(), 41_760);
    let mut file = create(s.root(), path);
    for day in all.chunks(1_440) {
        assert!(matches!(file.append(day), Ok(Appended::Committed { .. })));
    }
    assert_eq!(file.records(), 41_760);
    assert_eq!(file.header().last_ts_micros, ist(2024, 2, 29, 23, 59));
    let before = image(s.root(), path);
    assert_eq!(
        u64::try_from(before.len()).unwrap(),
        HEADER_LEN + 41_760 * RECORD_STRIDE,
        "the file is exactly its header region and its records"
    );
    let next = bar(ist(2024, 3, 1, 0, 0), 0);
    assert!(
        matches!(file.append(&[next]), Err(StoreError::OutsideMonth { .. })),
        "the first minute of March is not February's"
    );
    assert_eq!(image(s.root(), path), before, "a refusal writes nothing");
    drop(file);

    let read = BarFile::open_existing(s.root(), path, SYMBOL).expect("a full month reopens");
    assert_eq!(read.records(), 41_760);
    for index in [0_u64, 1, 72, 73, 20_879, 41_686, 41_758, 41_759] {
        assert_eq!(
            read.read_record(index),
            Ok(all[usize::try_from(index).unwrap()]),
            "record {index}"
        );
    }
    assert_eq!(read.first_at_or_after(ist(2024, 2, 29, 23, 59)), Ok(41_759));
    drop(read);
    let audited = audit(s.root(), path);
    assert!(audited.is_ok(), "the full month audits clean: {audited:?}");
}

/// The same 41,760 bars written as ONE batch and as 29 hold the same records
/// region and the same checksum sidecar: how a caller cut the input never
/// changes what the month holds.
#[test]
fn one_huge_batch_and_twenty_nine_daily_batches_hold_the_same_records_and_checksums() {
    let all = every_minute_of_leap_february();
    let path = month(2024, 2);

    let one = Scratch::new("huge-one");
    let mut file = create(one.root(), path);
    assert_eq!(
        file.append(&all),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 41_760
        })
    );
    drop(file);

    let many = Scratch::new("huge-many");
    let mut file = create(many.root(), path);
    for day in all.chunks(1_440) {
        assert!(matches!(file.append(day), Ok(Appended::Committed { .. })));
    }
    drop(file);

    let a = image(one.root(), path);
    let b = image(many.root(), path);
    let header = usize::try_from(HEADER_LEN).unwrap();
    assert_eq!(a.len(), b.len());
    assert!(
        a[header..] == b[header..],
        "the records region is identical"
    );
    let crc = path.with_file(FileKind::Checksums);
    assert_eq!(
        fs::read(crc.to_path_buf(one.root())).expect("a checksum sidecar"),
        fs::read(crc.to_path_buf(many.root())).expect("a checksum sidecar"),
        "the checksum sidecar is a function of the records alone"
    );
}

// ===========================================================================
// Duplicates
// ===========================================================================

/// A batch that carries one stamp twice is refused WHOLE, whether the two
/// copies agree byte for byte or not, and leaves the month untouched; the
/// same bars without the repeat then commit.
#[test]
fn a_batch_repeating_a_stamp_is_refused_whole_whether_or_not_the_copies_agree() {
    let s = Scratch::new("dup-in-batch");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let mut file = create(s.root(), path);
    let before = month_files(s.root(), path);

    let same = [
        bar(t, 0),
        bar(t + MINUTE, 1),
        bar(t + MINUTE, 1),
        bar(t + 2 * MINUTE, 2),
    ];
    let differ = [
        bar(t, 0),
        bar(t + MINUTE, 1),
        bar(t + MINUTE, 7),
        bar(t + 2 * MINUTE, 2),
    ];
    for batch in [&same[..], &differ[..]] {
        assert!(
            matches!(
                file.append(batch),
                Err(StoreError::BatchNotOrdered { at: 2, .. })
            ),
            "the second copy of the stamp is named"
        );
        assert_eq!(month_files(s.root(), path), before, "nothing was written");
        assert_eq!(file.records(), 0);
    }
    let clean = [bar(t, 0), bar(t + MINUTE, 1), bar(t + 2 * MINUTE, 2)];
    assert_eq!(
        file.append(&clean),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 3
        })
    );
    // And a held stamp offered again inside a batch that ALSO repeats it.
    let again = [bar(t + MINUTE, 1), bar(t + MINUTE, 1)];
    assert!(matches!(
        file.append(&again),
        Err(StoreError::BatchNotOrdered { .. })
    ));
    assert_eq!(file.records(), 3);
}

/// Late bars that fall BETWEEN held ones (the month holds every other minute)
/// are refused, alone or mixed with held bars, and nothing moves; the next
/// minute after the last held one still commits.
#[test]
fn late_bars_interleaved_with_held_ones_are_refused_and_change_nothing() {
    let s = Scratch::new("interleave");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let held: Vec<Bar> = (0..11).map(|i| bar(t + 2 * i * MINUTE, i)).collect();
    let mut file = create(s.root(), path);
    assert!(matches!(file.append(&held), Ok(Appended::Committed { .. })));
    let before = month_files(s.root(), path);

    let late_alone = [bar(t + MINUTE, 50), bar(t + 3 * MINUTE, 51)];
    let late_mixed = [held[0], bar(t + MINUTE, 50), held[1]];
    let late_last = [bar(t + 19 * MINUTE, 52)];
    for batch in [&late_alone[..], &late_mixed[..], &late_last[..]] {
        let got = file.append(batch);
        assert!(
            matches!(got, Err(StoreError::OverlapDisagrees { .. })),
            "a late bar is a conflict by name, got {got:?}"
        );
        assert_eq!(month_files(s.root(), path), before, "nothing was written");
        assert_eq!(file.records(), 11);
    }
    assert_eq!(
        file.append(&[bar(t + 21 * MINUTE, 60)]),
        Ok(Appended::Committed {
            first_index: 11,
            n_valid: 12
        })
    );
}

/// A resumed superset whose overlap matches appends only its new tail, and the
/// month it leaves is byte-identical to one written without the overlap.
#[test]
fn a_resumed_superset_appends_only_its_tail_and_lands_the_clean_bytes() {
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let all = minutes(t, 150);

    let resumed = Scratch::new("superset");
    let mut file = create(resumed.root(), path);
    assert!(matches!(
        file.append(&all[..100]),
        Ok(Appended::Committed { .. })
    ));
    assert_eq!(
        file.append(&all[50..]),
        Ok(Appended::Committed {
            first_index: 100,
            n_valid: 150
        })
    );
    drop(file);

    let clean = Scratch::new("superset-clean");
    let mut file = create(clean.root(), path);
    assert!(matches!(
        file.append(&all[..100]),
        Ok(Appended::Committed { .. })
    ));
    assert!(matches!(
        file.append(&all[100..]),
        Ok(Appended::Committed { .. })
    ));
    drop(file);

    assert_eq!(
        month_files(resumed.root(), path),
        month_files(clean.root(), path),
        "resuming over an overlap lands the bytes a clean run lands"
    );
}

/// A superset whose overlap is wrong by ONE paisa in one bar is refused whole:
/// not one of its new bars is appended.
#[test]
fn a_superset_whose_overlap_is_one_paisa_wrong_appends_nothing() {
    let s = Scratch::new("superset-wrong");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let all = minutes(t, 150);
    let mut file = create(s.root(), path);
    assert!(matches!(
        file.append(&all[..100]),
        Ok(Appended::Committed { .. })
    ));
    let before = month_files(s.root(), path);
    let mut offered = all[50..].to_vec();
    offered[25].close += 1;
    assert!(matches!(
        file.append(&offered),
        Err(StoreError::OverlapDisagrees { .. })
    ));
    assert_eq!(file.records(), 100);
    assert_eq!(month_files(s.root(), path), before, "nothing was written");
}

// ===========================================================================
// Zero and extreme values
// ===========================================================================

/// Zero volume and zero open interest are stored as zero, never as absent, and
/// the absent sentinel stays absent: `CLAUDE.md` section 7's "zero means zero".
#[test]
fn zero_volume_and_zero_interest_are_stored_as_zero_and_absent_stays_absent() {
    let s = Scratch::new("zero-counts");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let mut quiet = bar(t, 0);
    quiet.volume = 0;
    quiet.open_interest = 0;
    let mut absent = bar(t + MINUTE, 1);
    absent.volume = 0;
    let mut file = create(s.root(), path);
    assert!(matches!(
        file.append(&[quiet, absent]),
        Ok(Appended::Committed { .. })
    ));
    drop(file);
    let read = BarFile::open_existing(s.root(), path, SYMBOL).unwrap();
    let back = records(&read);
    assert_eq!(back[0].volume, 0);
    assert_eq!(back[0].oi(), Some(0), "zero interest is a value");
    assert_eq!(back[1].volume, 0);
    assert_eq!(back[1].oi(), None, "the sentinel is absent, not zero");
}

/// The largest values an `i64` holds are stored and read back exactly, and
/// two volumes whose SUM passes `i64::MAX` are either stored or refused by
/// name: the month's summary and indexes must not overflow on them.
#[test]
fn the_largest_values_round_trip_and_an_overflowing_volume_sum_is_not_a_panic() {
    let s = Scratch::new("extreme-values");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let huge = Bar {
        ts_micros: t,
        open: i64::MAX,
        high: i64::MAX,
        low: i64::MAX,
        close: i64::MAX,
        volume: i64::MAX,
        open_interest: i64::MAX,
    };
    let mut also = huge;
    also.ts_micros = t + MINUTE;
    let mut file = create(s.root(), path);
    let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| file.append(&[huge, also])));
    let got = got.expect("an append never panics on a legal bar");
    assert!(
        matches!(got, Ok(Appended::Committed { .. })),
        "two sane bars at the edge of i64 commit: {got:?}"
    );
    drop(file);
    let read = BarFile::open_existing(s.root(), path, SYMBOL).unwrap();
    assert_eq!(records(&read), vec![huge, also]);
}

/// FINDING (store, zero price).
///
/// Input: one bar of a NIFTY spot month whose open, high, low and close are
/// all `0` paisa, volume 10.
///
/// Expected: refused at the write boundary, like a negative price is
/// (`StoreError::ImpossibleBar`): nothing this store holds (a spot index, a
/// cash share, a future or an option) ever trades at zero, so a zero is a
/// decoder that read an empty column, or a vendor's "no trade" placeholder.
///
/// Actual: committed. `Bar::ohlc_is_sane` (`crates/store/src/format.rs:800`)
/// checks `>= 0`, so a zero passes, and `crates/pull/src/http.rs` documents
/// zero as "a legal price". A zero low inside a real month reads to every
/// indicator as a 100% crash in one minute.
///
/// Proposed fix: decide it (the zero rule is recorded as deliberate): either
/// refuse a zero price for every `Bar` (`> 0` in the sign clause), or refuse
/// it for the index and cash segments where it can never be real, and name the
/// refusal (`ImpossibleBar`) so the importer's day fails loudly.
#[test]
#[ignore = "finding: the store admits a bar priced at zero (EXTREMES-REPORT.md)"]
fn finding_a_bar_priced_at_zero_is_admitted_to_a_spot_month() {
    let s = Scratch::new("zero-price");
    let path = month(2024, 6);
    let t = ist(2024, 6, 3, 9, 15);
    let zero = Bar {
        ts_micros: t,
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 10,
        open_interest: OI_NULL,
    };
    let mut file = create(s.root(), path);
    let got = file.append(&[zero]);
    assert!(
        matches!(got, Err(StoreError::ImpossibleBar { at: 0 })),
        "a zero-priced bar must be refused at the write boundary, got {got:?}"
    );
    assert_eq!(file.records(), 0);
}

// ===========================================================================
// The leap day, Februaries without one, and the year boundary
// ===========================================================================

/// 29 February 2024 belongs to February alone: its open and its last minute
/// commit there, March refuses them, and February refuses March's first
/// instant.
#[test]
fn the_leap_day_belongs_to_february_2024_alone() {
    let feb = Scratch::new("leap-feb");
    let mar = Scratch::new("leap-mar");
    let open = bar(ist(2024, 2, 29, 9, 15), 0);
    let last = bar(ist(2024, 2, 29, 23, 59), 1);
    let first_of_march = bar(ist(2024, 3, 1, 0, 0), 2);

    let mut file = create(feb.root(), month(2024, 2));
    assert!(matches!(
        file.append(&[open, last]),
        Ok(Appended::Committed { n_valid: 2, .. })
    ));
    assert!(matches!(
        file.append(&[first_of_march]),
        Err(StoreError::OutsideMonth { .. })
    ));

    let mut file = create(mar.root(), month(2024, 3));
    assert!(matches!(
        file.append(&[open]),
        Err(StoreError::OutsideMonth { .. })
    ));
    assert!(matches!(
        file.append(&[first_of_march]),
        Ok(Appended::Committed { n_valid: 1, .. })
    ));
}

/// A February with no 29th spans exactly 28 days of IST, including 2100,
/// which the four-year rule alone would make a leap year; 2000 and 2024 span
/// 29. The instant a naive "29 February 2023" would map to (1 March) is
/// refused by February 2023.
#[test]
fn a_february_without_a_twenty_ninth_is_twenty_eight_ist_days_including_2100() {
    for (year, days) in [
        (2023_u16, 28_i64),
        (2100, 28),
        (2000, 29),
        (2024, 29),
        (1970, 28),
    ] {
        let (from, until) = YearMonth::new(year, 2).unwrap().ist_bounds_micros();
        assert_eq!(until - from, days * DAY, "February {year}");
        assert_eq!(
            from,
            ist(i64::from(year), 2, 1, 0, 0),
            "February {year} starts"
        );
    }
    let s = Scratch::new("feb-2023");
    let mut file = create(s.root(), month(2023, 2));
    assert!(matches!(
        file.append(&[bar(ist(2023, 2, 28, 23, 59), 0)]),
        Ok(Appended::Committed { .. })
    ));
    assert!(matches!(
        file.append(&[bar(ist(2023, 3, 1, 0, 0), 1)]),
        Err(StoreError::OutsideMonth { .. })
    ));
}

/// The year boundary is IST midnight, not UTC midnight: 03:00 IST on 1
/// January 2025 is still 31 December in UTC, and it is January's.
#[test]
fn the_year_boundary_is_drawn_at_ist_midnight_not_utc() {
    let dec = Scratch::new("year-dec");
    let jan = Scratch::new("year-jan");
    let last_of_year = bar(ist(2024, 12, 31, 23, 59), 0);
    let first_of_year = bar(ist(2025, 1, 1, 0, 0), 1);
    let still_december_in_utc = bar(ist(2025, 1, 1, 3, 0), 2);
    assert!(
        still_december_in_utc.ts_micros < days_from_civil(2025, 1, 1) * DAY,
        "the fixture is before UTC midnight"
    );

    let mut file = create(dec.root(), month(2024, 12));
    assert!(matches!(
        file.append(&[last_of_year]),
        Ok(Appended::Committed { .. })
    ));
    for bar in [first_of_year, still_december_in_utc] {
        assert!(matches!(
            file.append(&[bar]),
            Err(StoreError::OutsideMonth { .. })
        ));
    }
    let mut file = create(jan.root(), month(2025, 1));
    assert!(matches!(
        file.append(&[last_of_year]),
        Err(StoreError::OutsideMonth { .. })
    ));
    assert!(matches!(
        file.append(&[first_of_year, still_december_in_utc]),
        Ok(Appended::Committed { n_valid: 2, .. })
    ));
}

// ===========================================================================
// The same input twice
// ===========================================================================

/// Three days of bars in three batches.
fn three_days() -> Vec<Vec<Bar>> {
    [3_i64, 4, 5]
        .iter()
        .map(|&d| minutes(ist(2024, 6, d, 9, 15), 375))
        .collect()
}

/// The same batches written into two fresh roots leave every file of the
/// month byte-identical: no wall clock, process id or path reaches the bytes.
#[test]
fn the_same_batches_in_two_fresh_roots_leave_identical_month_files() {
    let path = month(2024, 6);
    let a = Scratch::new("twice-a");
    let b = Scratch::new("twice-b");
    for root in [a.root(), b.root()] {
        let mut file = create(root, path);
        for day in three_days() {
            assert!(matches!(file.append(&day), Ok(Appended::Committed { .. })));
        }
    }
    let first = month_files(a.root(), path);
    assert!(
        first.len() >= 2,
        "the bars and their checksums at least: {:?}",
        first.keys()
    );
    assert_eq!(first, month_files(b.root(), path));
}

/// Writing the whole input a second time, through a fresh handle, answers
/// every batch `AlreadyPresent` at its own index and changes no byte of any
/// file of the month, sidecars included.
#[test]
fn writing_the_whole_input_again_after_a_reopen_changes_no_byte_of_any_file() {
    let s = Scratch::new("rerun");
    let path = month(2024, 6);
    let days = three_days();
    let mut file = create(s.root(), path);
    for day in &days {
        assert!(matches!(file.append(day), Ok(Appended::Committed { .. })));
    }
    drop(file);
    let before = month_files(s.root(), path);

    let mut file = create(s.root(), path);
    for (n, day) in days.iter().enumerate() {
        assert_eq!(
            file.append(day),
            Ok(Appended::AlreadyPresent {
                first_index: u64::try_from(n).unwrap() * 375,
                n_valid: 1_125
            })
        );
    }
    // The middle day alone, and the whole month in one batch.
    assert!(matches!(
        file.append(&days[1]),
        Ok(Appended::AlreadyPresent {
            first_index: 375,
            ..
        })
    ));
    let whole: Vec<Bar> = days.concat();
    assert!(matches!(
        file.append(&whole),
        Ok(Appended::AlreadyPresent { first_index: 0, .. })
    ));
    drop(file);
    assert_eq!(month_files(s.root(), path), before);
}

/// The full checksum audit of a month, reduced to its verdict.
fn audit(root: &Path, path: StorePath<'_>) -> Result<(), String> {
    BarFile::open_existing_audited(root, path, SYMBOL, u64::MAX).map(drop)
}
