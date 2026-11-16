//! The month repair's row ceiling and its step between publication and
//! promotion (D-4875 (1) and (5), D-4898; ingest automation package IA-17).
//!
//! Every store here is a temporary directory of invented bars: a fixture
//! symbol, and prices near 1,234 rupees that no index or share in these
//! months traded at. Nothing reads or writes a real store.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::generation::{self, Served};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::repair::{
    self, MAX_ROWS, MonthRepair, Policy, Published, RepairError, Revision, RevisionReader,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

const SYMBOL: u32 = 23;
/// An index or a share: open interest is not a measurement, and the vendor
/// does not re-base its prices (GDFL's policy, D-4875 (4)).
const AS_PUBLISHED: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};
const DAY: i64 = 86_400;

struct Store(PathBuf);

impl Store {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-month-repair-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    /// Appends `bars` to the original month, one batch.
    fn hold(&self, path: StorePath<'_>, bars: &[Bar]) {
        let mut writer = BarFile::open_or_create(&self.0, path, SYMBOL).unwrap();
        writer.append(bars).unwrap();
    }

    fn served(&self, path: StorePath<'_>) -> u32 {
        generation::served(&self.0, path).unwrap().ordinal()
    }

    fn records(&self, path: StorePath<'_>) -> u64 {
        BarFile::open_existing(&self.0, path, SYMBOL)
            .unwrap()
            .header()
            .n_valid
    }

    /// Every file under the store, by its path beneath the root.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut pending = vec![self.0.clone()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(&directory).unwrap() {
                let at = entry.unwrap().path();
                if at.is_dir() {
                    pending.push(at);
                } else {
                    let name = at.strip_prefix(&self.0).unwrap().to_path_buf();
                    found.insert(name, fs::read(&at).unwrap());
                }
            }
        }
        found
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn july() -> YearMonth {
    YearMonth::new(2024, 7).unwrap()
}

fn path_of(segment: &'static str, timeframe: Timeframe, month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol: if segment == "CASH" {
            "FIXTURECO"
        } else {
            "FIXTUREIDX"
        },
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The invented bar at the month's `slot`-th place on its rung's grid. The
/// grid is counted from the 09:15 IST open (D-0915), so its first place in a
/// day is 555 minutes modulo the rung after IST midnight: midnight itself on
/// the rungs that divide 555 minutes, one minute past it on the two-minute
/// rung, fifteen past on the hourly one.
fn bar_at(path: StorePath<'_>, slot: i64) -> Bar {
    let (from, _) = path.month().ist_bounds_micros();
    let width = i64::from(path.timeframe().secs());
    let price = 123_400 + slot % 97;
    Bar {
        ts_micros: from + ((555 * 60) % width + slot * width) * 1_000_000,
        open: price,
        high: price + 4,
        low: price - 4,
        close: price + 1,
        volume: 0,
        open_interest: OI_NULL,
    }
}

fn slots(path: StorePath<'_>, range: std::ops::Range<i64>) -> Vec<Bar> {
    range.map(|slot| bar_at(path, slot)).collect()
}

fn auction_file(revision_root: &Path, path: StorePath<'_>) -> PathBuf {
    path.to_path_buf(revision_root).with_extension("auc")
}

fn repaired(outcome: Result<MonthRepair, RepairError>) -> (u32, u32, u64, u64, Published, usize) {
    match outcome {
        Ok(MonthRepair::Repaired {
            from,
            to,
            held,
            added,
            published,
            skipped,
            ..
        }) => (from, to.ordinal(), held, added, published, skipped.len()),
        other => panic!("not a repair: {other:?}"),
    }
}

/// IA-RP-01. A one-second month repairs AT the grid ceiling: 30 held days of
/// every second and the one day it lacks merge to 2,678,400 rows, the most a
/// 31-day month holds, and publish as a generation every reader opens. Its
/// `.tix` is there and answers time to row through the index, no `.prv` is
/// written (D-4887), and one row more is refused `RowLimit`, naming both
/// counts, at both doors.
#[test]
fn a_full_1s_month_repairs_with_its_sidecars() {
    const LACKED: i64 = 17;
    let store = Store::new();
    let seconds = path_of("INDEX", Timeframe::SECOND_1, july());
    let ceiling = repair::row_ceiling(seconds);
    assert_eq!(ceiling, 31 * 86_400);
    assert_eq!(ceiling, MAX_ROWS as u64);
    {
        let mut writer = BarFile::open_or_create(&store.0, seconds, SYMBOL).unwrap();
        for day in (0..31).filter(|day| *day != LACKED) {
            writer
                .append(&slots(seconds, day * DAY..(day + 1) * DAY))
                .unwrap();
        }
        assert_eq!(writer.header().n_valid, 30 * 86_400);
    }
    let original = seconds.to_path_buf(&store.0);
    let before = (
        fs::metadata(&original).unwrap().len(),
        store::crc::crc32c(&fs::read(&original).unwrap()),
    );
    let lacked = slots(seconds, LACKED * DAY..(LACKED + 1) * DAY);

    let outcome = repair::repair_month(&store.0, seconds, SYMBOL, &lacked, AS_PUBLISHED);
    assert_eq!(
        repaired(outcome),
        (0, 1, 30 * 86_400, 86_400, Published::Created, 0)
    );

    // THE ORIGINAL IS BYTE-IDENTICAL, and every reader opens the full month.
    assert_eq!(
        (
            fs::metadata(&original).unwrap().len(),
            store::crc::crc32c(&fs::read(&original).unwrap()),
        ),
        before
    );
    let served = BarFile::open_existing(&store.0, seconds, SYMBOL).unwrap();
    assert_eq!(served.header().n_valid, ceiling);
    // WRITTEN IN SIX COMMITS, NOT ONE (D-4898, L-IA-26): five more than a
    // month one append wrote, so what the writer held per commit was a
    // commit's rows and never the month's.
    let one_commit = {
        let other = Store::new();
        let minutes = path_of("INDEX", Timeframe::MINUTE_1, july());
        other.hold(minutes, &slots(minutes, 0..1));
        BarFile::open_existing(&other.0, minutes, SYMBOL)
            .unwrap()
            .header()
            .generation
    };
    assert_eq!(served.header().generation, one_commit + 5);
    for slot in (0..31 * DAY).step_by(9_973) {
        assert_eq!(
            served.read_record(u64::try_from(slot).unwrap()).unwrap(),
            bar_at(seconds, slot)
        );
    }
    // ITS `.tix` IS PRESENT AND IS WHAT ANSWERS: every second of the month is
    // held, so the row of a stamp is its second, at both ends of the month,
    // at both ends of the day that was added, and one past the last bar.
    let revision_root = repair::revision_root(&store.0, Revision::new(1).unwrap());
    assert!(
        seconds
            .with_file(FileKind::TimeIndex)
            .to_path_buf(&revision_root)
            .is_file()
    );
    assert_eq!(served.time_lookup(), TimeLookup::Indexed);
    for slot in [
        0,
        LACKED * DAY - 1,
        LACKED * DAY,
        (LACKED + 1) * DAY - 1,
        (LACKED + 1) * DAY,
        31 * DAY - 1,
        31 * DAY,
    ] {
        assert_eq!(
            served
                .first_at_or_after(bar_at(seconds, slot).ts_micros)
                .unwrap(),
            u64::try_from(slot).unwrap(),
            "second {slot}"
        );
    }
    // NO `.prv`: a revision's records would index the original's bars.
    assert!(
        !seconds
            .with_file(FileKind::Provenance)
            .to_path_buf(&revision_root)
            .exists()
    );
    drop(served);

    one_row_more_is_refused_at_both_doors(&store, seconds, ceiling);
}

/// One row past a full one-second month's ceiling is refused by name, with
/// both counts, at both doors, and before anything is read or written; the
/// month offered again whole, at the ceiling, is all held.
fn one_row_more_is_refused_at_both_doors(store: &Store, seconds: StorePath<'_>, ceiling: u64) {
    let mut over = slots(seconds, 0..31 * DAY + 1);
    let refused = RepairError::RowLimit {
        rows: ceiling + 1,
        ceiling,
    };
    assert_eq!(
        repair::repair_month(&store.0, seconds, SYMBOL, &over, AS_PUBLISHED),
        Err(refused.clone())
    );
    let header = BarFile::open_existing(&store.0, seconds, SYMBOL)
        .unwrap()
        .header();
    assert_eq!(
        repair::publish(
            &store.0,
            seconds,
            SYMBOL,
            Revision::new(2).unwrap(),
            header,
            &over
        ),
        Err(refused)
    );
    // AT THE CEILING the count passes: the whole month offered again is all
    // held, and nothing is written.
    over.pop();
    assert_eq!(
        repair::repair_month(&store.0, seconds, SYMBOL, &over, AS_PUBLISHED),
        Ok(MonthRepair::NothingNew {
            ordinal: 1,
            n_valid: ceiling
        })
    );
    assert_eq!(store.served(seconds), 1);
    assert!(
        !repair::revision_root(&store.0, Revision::new(2).unwrap()).exists(),
        "a refusal reserved an ordinal"
    );
}

/// The ceiling is the path's own month on its own rung, at every rung the
/// store files and every month length: one row past it is refused `RowLimit`
/// before any bar is looked at, where one 2,678,400 for every rung let a
/// one-minute request of 44,641 rows through to be walked.
#[test]
fn the_row_limit_is_the_paths_own_month_on_its_own_rung() {
    let months = [(2023, 2, 28), (2024, 2, 29), (2024, 6, 30), (2024, 7, 31)];
    for timeframe in Timeframe::KNOWN.iter().copied() {
        for (year, month, days) in months {
            let path = path_of("INDEX", timeframe, YearMonth::new(year, month).unwrap());
            let ceiling = repair::row_ceiling(path);
            assert_eq!(
                ceiling,
                days * 86_400 / u64::from(timeframe.secs()),
                "{} {year}-{month}",
                timeframe.as_str()
            );
            assert!(ceiling <= MAX_ROWS as u64);
            if timeframe == Timeframe::SECOND_1 {
                // 2.4 to 2.7 million rows: `a_full_1s_month_repairs_with_its_sidecars`.
                continue;
            }
            let store = Store::new();
            store.hold(path, &slots(path, 0..1));
            // GARBAGE ON PURPOSE: stamps on a one-second spacing, off this
            // rung's grid. The count refuses first, so they are never read.
            let (from, _) = path.month().ist_bounds_micros();
            let over: Vec<Bar> = (0..=i64::try_from(ceiling).unwrap())
                .map(|second| Bar {
                    ts_micros: from + second * 1_000_000,
                    ..bar_at(path, 0)
                })
                .collect();
            let refused = RepairError::RowLimit {
                rows: ceiling + 1,
                ceiling,
            };
            assert_eq!(
                repair::repair_month(&store.0, path, SYMBOL, &over, AS_PUBLISHED),
                Err(refused.clone())
            );
            let header = BarFile::open_existing(&store.0, path, SYMBOL)
                .unwrap()
                .header();
            assert_eq!(
                repair::publish(
                    &store.0,
                    path,
                    SYMBOL,
                    Revision::new(1).unwrap(),
                    header,
                    &over
                ),
                Err(refused)
            );
            // A SOURCE HEADER COUNTING ONE MORE THAN THE MONTH HOLDS.
            let mut forged = header;
            forged.n_valid = ceiling + 1;
            assert_eq!(
                repair::publish(
                    &store.0,
                    path,
                    SYMBOL,
                    Revision::new(1).unwrap(),
                    forged,
                    &slots(path, 0..2)
                ),
                Err(RepairError::RowLimit {
                    rows: ceiling + 1,
                    ceiling
                })
            );
            assert!(!store.0.join("bar-revisions-v1").exists());
            assert_eq!(store.served(path), 0);
        }
    }
}

/// A month that is FULL on its rung still repairs: every minute of July but
/// one, and every day of July but one, each take the bar they lack and are
/// served whole, at exactly the ceiling. A 32nd daily bar is refused.
#[test]
fn a_full_month_on_a_coarse_rung_takes_its_last_bar() {
    for timeframe in [Timeframe::MINUTE_1, Timeframe::DAY_1] {
        let store = Store::new();
        let path = path_of("INDEX", timeframe, july());
        let ceiling = i64::try_from(repair::row_ceiling(path)).unwrap();
        let lacked = ceiling / 2;
        let held: Vec<Bar> = (0..ceiling)
            .filter(|slot| *slot != lacked)
            .map(|slot| bar_at(path, slot))
            .collect();
        store.hold(path, &held);
        let outcome = repair::repair_month(
            &store.0,
            path,
            SYMBOL,
            &slots(path, lacked..lacked + 1),
            AS_PUBLISHED,
        );
        let kept = u64::try_from(ceiling - 1).unwrap();
        assert_eq!(repaired(outcome), (0, 1, kept, 1, Published::Created, 0));
        let served = BarFile::open_existing(&store.0, path, SYMBOL).unwrap();
        assert_eq!(served.header().n_valid, kept + 1);
        assert_eq!(
            served.read_record(u64::try_from(lacked).unwrap()).unwrap(),
            bar_at(path, lacked)
        );
    }
    // THE DAILY RUNG ADMITS ANY WHOLE SECOND, so 32 bars fit July's grid; a
    // month has 31 days, and the repair refuses the 32nd by count.
    let store = Store::new();
    let daily = path_of("INDEX", Timeframe::DAY_1, july());
    store.hold(daily, &slots(daily, 0..1));
    let (from, _) = daily.month().ist_bounds_micros();
    let thirty_two: Vec<Bar> = (0..32)
        .map(|half_day| Bar {
            ts_micros: from + half_day * 43_200 * 1_000_000,
            ..bar_at(daily, 0)
        })
        .collect();
    assert_eq!(
        repair::repair_month(&store.0, daily, SYMBOL, &thirty_two, AS_PUBLISHED),
        Err(RepairError::RowLimit {
            rows: 32,
            ceiling: 31
        })
    );
}

/// What a step records about the instant it ran in.
#[derive(Debug, Default, PartialEq, Eq)]
struct Seen {
    calls: u32,
    roots: Vec<PathBuf>,
    /// The generation readers were served while the step ran.
    served: Vec<u32>,
    /// What a reader of the month counted while the step ran.
    reader_rows: Vec<u64>,
    /// What the published revision counted while the step ran.
    revision_rows: Vec<u64>,
}

/// A CASH one-second month of two short held sessions, and the session it
/// lacks between them.
fn cash_month(store: &Store) -> (StorePath<'static>, Vec<Bar>) {
    let cash = path_of("CASH", Timeframe::SECOND_1, july());
    let open = 555 * 60;
    let mut held = slots(cash, 2 * DAY + open..2 * DAY + open + 600);
    held.extend(slots(cash, 4 * DAY + open..4 * DAY + open + 600));
    store.hold(cash, &held);
    (cash, slots(cash, 3 * DAY + open..3 * DAY + open + 600))
}

fn look(store: &Store, path: StorePath<'_>, seen: &mut Seen, revision_root: &Path) {
    let ordinal: u32 = revision_root
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
        .expect("the step is handed a revision root");
    seen.calls += 1;
    seen.roots.push(revision_root.to_path_buf());
    seen.served.push(store.served(path));
    seen.reader_rows.push(store.records(path));
    seen.revision_rows.push(
        RevisionReader::open(&store.0, path, SYMBOL, Revision::new(ordinal).unwrap())
            .expect("the revision is published and whole when the step runs")
            .header()
            .n_valid,
    );
}

/// IA-RP-02. A CASH repair's auction file is written by the step
/// `repair_month_with` runs between publication and promotion: when the step
/// runs the revision is whole and no reader is served from it; a step that
/// fails leaves it published and unserved, its words kept; the exact retry
/// reuses the revision, runs the step again and promotes; and a caller that
/// passes no step writes the bytes `repair_month` writes.
#[test]
fn a_cash_repair_writes_its_auction_file_before_it_is_served() {
    const WHOLE: &[u8] = b"FIXTURECO auction 2024-07-03 open 1234.00\nFIXTURECO auction 2024-07-04 open 1235.00\nFIXTURECO auction 2024-07-05 open 1236.00\n";
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    let first = repair::revision_root(&store.0, Revision::new(1).unwrap());
    let mut seen = Seen::default();

    // THE FIRST ATTEMPT'S STEP FAILS, with half its file written: a full
    // disk, or a process killed while it wrote.
    let mut failing = |revision_root: &Path| {
        look(&store, cash, &mut seen, revision_root);
        fs::write(auction_file(revision_root, cash), &WHOLE[..40]).unwrap();
        Err("auction file: No space left on device (os error 28)".to_owned())
    };
    let refused = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut failing),
    );
    assert_eq!(
        refused,
        Err(RepairError::BeforePromote {
            revision_root: first.clone(),
            why: "auction file: No space left on device (os error 28)".to_owned(),
        })
    );
    let said = refused.unwrap_err().to_string();
    assert!(
        said.contains("No space left on device (os error 28)"),
        "{said}"
    );
    assert!(said.contains("published and not served"), "{said}");
    // PUBLISHED AND UNSERVED: readers still read the original's 1,200 bars,
    // the revision of 1,800 is whole, and nothing sits beside the original.
    assert_eq!(store.served(cash), 0);
    assert_eq!(store.records(cash), 1_200);
    assert_eq!(
        RevisionReader::open(&store.0, cash, SYMBOL, Revision::new(1).unwrap())
            .unwrap()
            .header()
            .n_valid,
        1_800
    );
    assert!(!auction_file(&store.0, cash).exists());
    assert_eq!(fs::read(auction_file(&first, cash)).unwrap(), &WHOLE[..40]);

    // THE EXACT RETRY reuses the revision, runs the step again over its own
    // torn file, and promotes.
    let mut writing = |revision_root: &Path| {
        look(&store, cash, &mut seen, revision_root);
        fs::write(auction_file(revision_root, cash), WHOLE).map_err(|why| why.to_string())
    };
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut writing),
    );
    assert_eq!(repaired(outcome), (0, 1, 1_200, 600, Published::Reused, 0));
    // BOTH TIMES THE STEP RAN BEFORE THE MONTH WAS SERVED FROM THE REVISION.
    assert_eq!(
        seen,
        Seen {
            calls: 2,
            roots: vec![first.clone(), first.clone()],
            served: vec![0, 0],
            reader_rows: vec![1_200, 1_200],
            revision_rows: vec![1_800, 1_800],
        }
    );
    // SERVED NOW, and the auction file is beside the served bars.
    assert_eq!(store.served(cash), 1);
    assert_eq!(store.records(cash), 1_800);
    let beside_served = generation::served_bars_path(&store.0, cash)
        .unwrap()
        .with_extension("auc");
    assert_eq!(beside_served, auction_file(&first, cash));
    assert_eq!(fs::read(beside_served).unwrap(), WHOLE);
    // OFFERED A THIRD TIME, everything is held: nothing is written and the
    // step is not asked.
    let mut never = |_: &Path| -> Result<(), String> { panic!("nothing is about to be promoted") };
    assert_eq!(
        repair::repair_month_with(
            &store.0,
            cash,
            SYMBOL,
            &lacked,
            AS_PUBLISHED,
            Some(&mut never)
        ),
        Ok(MonthRepair::NothingNew {
            ordinal: 1,
            n_valid: 1_800
        })
    );

    no_step_is_the_wrapper_byte_for_byte(cash);
}

/// No step is `repair_month`, byte for byte: every file of the revision and
/// of the original is the same either way. The pointer record carries the
/// promotion's wall-clock time, so it is compared by what it serves.
fn no_step_is_the_wrapper_byte_for_byte(cash: StorePath<'_>) {
    let (plain, with_none) = (Store::new(), Store::new());
    let (_, offered) = cash_month(&plain);
    let _ = cash_month(&with_none);
    let through_wrapper = repair::repair_month(&plain.0, cash, SYMBOL, &offered, AS_PUBLISHED);
    let through_none =
        repair::repair_month_with(&with_none.0, cash, SYMBOL, &offered, AS_PUBLISHED, None);
    assert_eq!(repaired(through_wrapper), repaired(through_none));
    let pointer = generation::pointer_path(&plain.0, cash);
    let pointer = pointer.strip_prefix(&plain.0).unwrap();
    let (mut left, mut right) = (plain.files(), with_none.files());
    assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
    assert_eq!(
        left.keys().collect::<Vec<_>>(),
        right.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in &left {
        assert!(right.get(name) == Some(bytes), "{} differs", name.display());
    }
    assert!(
        left.keys()
            .all(|name| name.extension().is_none_or(|e| e != "auc"))
    );
    assert_eq!(
        generation::served(&plain.0, cash).unwrap().ordinal(),
        generation::served(&with_none.0, cash).unwrap().ordinal()
    );
}

/// The step is asked only for a revision about to be promoted: never when
/// every offered bar is held, never for an empty offer, and never behind a
/// refusal, each of which writes nothing.
#[test]
fn the_step_is_not_asked_when_nothing_is_about_to_be_promoted() {
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    let held = BarFile::open_existing(&store.0, cash, SYMBOL)
        .unwrap()
        .read_record(0)
        .unwrap();
    let mut calls = 0u32;
    let mut counting = |_: &Path| {
        calls += 1;
        Ok(())
    };
    let mut ask = |path: StorePath<'_>, offered: &[Bar], policy: Policy| {
        repair::repair_month_with(&store.0, path, SYMBOL, offered, policy, Some(&mut counting))
    };
    // EMPTY IS REFUSED BY NAME, as every write boundary refuses it; A SINGLE
    // BAR ALREADY HELD is nothing new.
    assert_eq!(
        ask(cash, &[], AS_PUBLISHED),
        Err(RepairError::Store(StoreError::EmptyBatch))
    );
    assert_eq!(
        ask(cash, &[held], AS_PUBLISHED),
        Ok(MonthRepair::NothingNew {
            ordinal: 0,
            n_valid: 1_200,
        })
    );
    // A HELD BAR THAT WOULD CHANGE.
    let changed = Bar {
        close: held.close + 1,
        ..held
    };
    assert!(matches!(
        ask(cash, &[changed], AS_PUBLISHED),
        Err(RepairError::HeldBarDiffers { .. })
    ));
    // OUT OF ORDER, AND A DUPLICATE STAMP.
    let mut backwards = lacked.clone();
    backwards.swap(0, 1);
    assert!(matches!(
        ask(cash, &backwards, AS_PUBLISHED),
        Err(RepairError::Store(StoreError::BatchNotOrdered { .. }))
    ));
    assert!(matches!(
        ask(cash, &[lacked[0], lacked[0]], AS_PUBLISHED),
        Err(RepairError::Store(StoreError::BatchNotOrdered { .. }))
    ));
    // ANOTHER MONTH'S BAR, AND A MONTH THAT DOES NOT EXIST.
    let august = path_of(
        "CASH",
        Timeframe::SECOND_1,
        YearMonth::new(2024, 8).unwrap(),
    );
    assert!(matches!(
        ask(cash, &slots(august, 0..1), AS_PUBLISHED),
        Err(RepairError::Store(StoreError::OutsideMonth { .. }))
    ));
    assert!(matches!(
        ask(august, &slots(august, 0..1), AS_PUBLISHED),
        Err(RepairError::Store(StoreError::Missing { .. }))
    ));
    // NO WITNESS OF THE PRICE BASIS, for a vendor that re-bases.
    let rebased = Policy {
        open_interest_applies: false,
        basis_witness: true,
    };
    assert_eq!(
        ask(cash, &lacked, rebased),
        Err(RepairError::NoBasisWitness)
    );
    // NOT A BAR MONTH.
    assert!(matches!(
        ask(cash.with_file(FileKind::Overlay), &lacked, AS_PUBLISHED),
        Err(RepairError::Store(StoreError::NotABarPath { .. }))
    ));
    assert_eq!(calls, 0);
    assert_eq!(store.served(cash), 0);
    assert!(!store.0.join("bar-revisions-v1").exists());
}

/// A promotion refused BEHIND the step (another promoter holds the month's
/// pointer) leaves the revision published and unserved, and the retry runs
/// the step again before it promotes: the step is repeatable by contract.
#[test]
fn a_promotion_refused_behind_the_step_runs_the_step_again_on_the_retry() {
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    let first = repair::revision_root(&store.0, Revision::new(1).unwrap());
    let pointer = generation::pointer_path(&store.0, cash);
    fs::create_dir_all(pointer.parent().unwrap()).unwrap();
    let other_promoter = fs::File::create(&pointer).unwrap();
    other_promoter.try_lock().expect("the pointer is free");
    let mut calls = 0u32;
    let mut writing = |revision_root: &Path| {
        calls += 1;
        fs::write(
            auction_file(revision_root, cash),
            format!("attempt {calls}\n"),
        )
        .map_err(|why| why.to_string())
    };
    let behind = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut writing),
    );
    assert!(
        matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
        "{behind:?}"
    );
    assert_eq!(store.served(cash), 0);
    assert_eq!(store.records(cash), 1_200);
    assert_eq!(
        fs::read(auction_file(&first, cash)).unwrap(),
        b"attempt 1\n"
    );
    other_promoter.unlock().unwrap();
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut writing),
    );
    assert_eq!(repaired(outcome), (0, 1, 1_200, 600, Published::Reused, 0));
    assert_eq!(calls, 2);
    assert_eq!(store.served(cash), 1);
    assert_eq!(
        fs::read(auction_file(&first, cash)).unwrap(),
        b"attempt 2\n"
    );
}

/// A step that never returns (a panic, standing in for a process killed while
/// it ran) holds no lock behind it: the month is still served as it was, and
/// the same repair offered again reuses the revision and promotes.
#[test]
fn a_step_that_dies_strands_nothing() {
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    let mut dying = |revision_root: &Path| -> Result<(), String> {
        fs::write(auction_file(revision_root, cash), b"half").unwrap();
        panic!("killed while the step ran")
    };
    let died = catch_unwind(AssertUnwindSafe(|| {
        repair::repair_month_with(
            &store.0,
            cash,
            SYMBOL,
            &lacked,
            AS_PUBLISHED,
            Some(&mut dying),
        )
    }));
    assert!(died.is_err());
    assert_eq!(store.served(cash), 0);
    assert_eq!(store.records(cash), 1_200);
    // A WRITER IS STILL REFUSED NOTHING IT WAS NOT REFUSED BEFORE: the
    // original takes an append, because it is still the served generation.
    let next_day = slots(cash, 5 * DAY..5 * DAY + 1);
    store.hold(cash, &next_day);
    assert_eq!(store.records(cash), 1_201);
    // THE MONTH MOVED ON, so the revision at ordinal 1 no longer describes
    // it: the retry steps over it, names it, and publishes ordinal 2.
    let mut writing = |revision_root: &Path| {
        fs::write(auction_file(revision_root, cash), b"whole\n").map_err(|why| why.to_string())
    };
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut writing),
    );
    assert_eq!(repaired(outcome), (0, 2, 1_201, 600, Published::Created, 1));
    assert_eq!(store.served(cash), 2);
    assert_eq!(store.records(cash), 1_801);
    // NEVER REUSED, NEVER OVERWRITTEN: the abandoned ordinal keeps its bytes.
    let first = repair::revision_root(&store.0, Revision::new(1).unwrap());
    assert_eq!(fs::read(auction_file(&first, cash)).unwrap(), b"half");
    let second = repair::revision_root(&store.0, Revision::new(2).unwrap());
    assert_eq!(fs::read(auction_file(&second, cash)).unwrap(), b"whole\n");
}

/// A DIFFERENT repair offered after a refused step does not inherit the
/// unserved revision: it is stepped over and named, the new request lands at
/// the next ordinal with the step handed THAT root, and the request refused
/// first then lands on top of it, late, every bar of all three kept.
#[test]
fn a_different_repair_after_a_refused_step_steps_over_the_unserved_revision() {
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    let mut refusing = |_: &Path| Err("not today".to_owned());
    assert!(matches!(
        repair::repair_month_with(
            &store.0,
            cash,
            SYMBOL,
            &lacked,
            AS_PUBLISHED,
            Some(&mut refusing)
        ),
        Err(RepairError::BeforePromote { .. })
    ));
    let mut roots = Vec::new();
    let mut noting = |revision_root: &Path| {
        roots.push(revision_root.to_path_buf());
        Ok(())
    };
    let later = slots(cash, 9 * DAY..9 * DAY + 300);
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &later,
        AS_PUBLISHED,
        Some(&mut noting),
    );
    let Ok(MonthRepair::Repaired {
        from, to, skipped, ..
    }) = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((from, to.ordinal()), (0, 2));
    assert_eq!(
        skipped,
        vec![(1, RepairError::Conflict.to_string())],
        "the unserved revision is named, never reused"
    );
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut noting),
    );
    assert_eq!(repaired(outcome), (2, 3, 1_500, 600, Published::Created, 0));
    assert_eq!(
        roots,
        [2, 3].map(|ordinal| repair::revision_root(&store.0, Revision::new(ordinal).unwrap()))
    );
    assert!(matches!(
        generation::served(&store.0, cash).unwrap(),
        Served::Revision(pointer) if pointer.revision.ordinal() == 3
    ));
    assert_eq!(store.records(cash), 2_100);
}

/// A REVISION CUT BETWEEN TWO OF ITS COMMITS (a process killed while a month
/// larger than one commit was being written, `WRITE_BATCH`, D-4898): what is
/// left at the ordinal is its reservation and a WELL-FORMED month file holding
/// only the first commit's bars, with no receipt. That is not a torn file, so
/// nothing but the missing receipt says it is unfinished, and the missing
/// receipt is enough: it is never served, never reopened and never completed
/// in place. The same repair offered again steps over it, names it, publishes
/// the whole month at the next ordinal and hands the step THAT root; the cut
/// revision keeps every byte it had.
#[test]
fn a_revision_cut_between_two_commits_is_stepped_over_and_kept() {
    let store = Store::new();
    let (cash, lacked) = cash_month(&store);
    // What `write_revision` has done when it is killed after its first commit
    // of a merged series it would have written in two: the reservation, then
    // the first bars through the store's own writer.
    let cut = repair::revision_root(&store.0, Revision::new(1).unwrap());
    let physical = cash.to_path_buf(&cut);
    fs::create_dir_all(physical.parent().unwrap()).unwrap();
    fs::write(physical.with_extension("reserved-v1"), b"").unwrap();
    let first_commit = slots(cash, 2 * DAY + 555 * 60..2 * DAY + 555 * 60 + 600);
    BarFile::open_or_create(&cut, cash, SYMBOL)
        .unwrap()
        .append(&first_commit)
        .unwrap();
    assert!(!physical.with_extension("repair-v1").exists());
    let left: BTreeMap<PathBuf, Vec<u8>> = store
        .files()
        .into_iter()
        .filter(|(name, _)| name.starts_with("bar-revisions-v1/1"))
        .collect();
    assert!(
        left.len() >= 2,
        "the cut revision's files: {:?}",
        left.keys()
    );

    // NOT A GENERATION: no reader opens it and the month is served as it was.
    assert!(matches!(
        RevisionReader::open(&store.0, cash, SYMBOL, Revision::new(1).unwrap()),
        Err(RepairError::Incomplete(_))
    ));
    assert_eq!(store.served(cash), 0);
    assert_eq!(store.records(cash), 1_200);

    let mut handed = Vec::new();
    let mut step = |revision_root: &Path| {
        handed.push(revision_root.to_path_buf());
        fs::write(auction_file(revision_root, cash), b"whole\n").map_err(|why| why.to_string())
    };
    let outcome = repair::repair_month_with(
        &store.0,
        cash,
        SYMBOL,
        &lacked,
        AS_PUBLISHED,
        Some(&mut step),
    );
    let skipped = match &outcome {
        Ok(MonthRepair::Repaired { skipped, .. }) => skipped.clone(),
        other => panic!("not a repair: {other:?}"),
    };
    assert_eq!(repaired(outcome), (0, 2, 1_200, 600, Published::Created, 1));
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].0, 1, "the cut ordinal is named: {skipped:?}");
    let second = repair::revision_root(&store.0, Revision::new(2).unwrap());
    assert_eq!(handed, vec![second.clone()]);
    assert_eq!(store.served(cash), 2);
    assert_eq!(store.records(cash), 1_800);
    assert_eq!(fs::read(auction_file(&second, cash)).unwrap(), b"whole\n");

    // EVERY BYTE THE CUT REVISION HAD, and nothing added beside it: the step
    // was not run there and no receipt was written for it afterwards.
    let now: BTreeMap<PathBuf, Vec<u8>> = store
        .files()
        .into_iter()
        .filter(|(name, _)| name.starts_with("bar-revisions-v1/1"))
        .collect();
    assert_eq!(now, left);
    assert!(matches!(
        RevisionReader::open(&store.0, cash, SYMBOL, Revision::new(1).unwrap()),
        Err(RepairError::Incomplete(_))
    ));
}
