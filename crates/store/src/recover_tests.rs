//! The month recovery door after a power cut (R3-20, D-7093): every state a
//! power cut can leave a month's day-end writes in, the named refusal and the
//! repaired generation of FM-062, and the extreme cases.
//!
//! Every store here is a temporary directory of invented bars: a fixture
//! symbol, and prices near 12,345 rupees that no index or share traded at in
//! these months. Nothing reads or writes a real store.
//!
//! # The write-recording seam
//!
//! A power cut is not a kill: the writes of a durable batch reach the drive
//! in any order between two flushes (D-3353). The seam here records a day's
//! writes as the bytes they change, by comparing the month's files before and
//! after the day's append: the file's new length, the records of each block
//! the append reached, each checksum entry it sealed, and the header slot.
//! Every subset of those writes is a state a power cut can leave, and every
//! one is rebuilt byte for byte and handed to the door.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test-only assertions over invented fixtures"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::{
    Basis, Cut, Dropped, Outcome, Plan, RECORD_LEN, Record, RecoverError, day_of, plan,
    read_record, record_path, recover, scoped, within,
};
use crate::file::{BarFile, StoreError};
use crate::format::{Bar, OI_NULL};
use crate::generation::{self, Served};
use crate::header::Header;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::repair::{self, MonthRepair, Policy, Published, Revision};

static NEXT: AtomicU64 = AtomicU64::new(0);

const SYMBOL: u32 = 41;
const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
/// 09:15 IST, past IST midnight.
const OPEN: i64 = 555 * MINUTE;
/// An index: open interest is not a measurement, prices are not re-based.
const INDEX: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};

/// A scratch store of invented bars, removed when it drops.
struct Store {
    root: PathBuf,
    month: YearMonth,
}

impl Store {
    fn new(year: u16, month: u8) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-recover-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self {
            root,
            month: YearMonth::new(year, month).unwrap(),
        }
    }

    fn path(&self) -> StorePath<'static> {
        self.path_of(FileKind::Bars)
    }

    fn path_of(&self, file: FileKind) -> StorePath<'static> {
        StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "FIXTURE",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: self.month,
            file,
        })
        .unwrap()
    }

    fn file(&self, kind: FileKind) -> PathBuf {
        self.path_of(kind).to_path_buf(&self.root)
    }

    /// One commit of `bars` to the original month.
    fn commit(&self, bars: &[Bar]) {
        let mut writer = BarFile::open_or_create(&self.root, self.path(), SYMBOL).unwrap();
        writer.append(bars).unwrap();
    }

    /// The month's three written files, as their bytes stand.
    fn image(&self) -> Image {
        Image {
            bin: fs::read(self.file(FileKind::Bars)).unwrap(),
            crc: fs::read(self.file(FileKind::Checksums)).unwrap_or_default(),
        }
    }

    /// Puts `image` back as the month's files.
    fn put(&self, image: &Image) {
        fs::write(self.file(FileKind::Bars), &image.bin).unwrap();
        fs::write(self.file(FileKind::Checksums), &image.crc).unwrap();
    }

    /// Every file under the store, by its path beneath the root.
    fn tree(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(&directory).unwrap() {
                let at = entry.unwrap().path();
                if at.is_dir() {
                    pending.push(at);
                } else {
                    let name = at.strip_prefix(&self.root).unwrap().to_path_buf();
                    found.insert(name, fs::read(&at).unwrap());
                }
            }
        }
        found
    }

    /// Every bar the served generation holds.
    fn served_bars(&self) -> Result<Vec<Bar>, StoreError> {
        let file = BarFile::open_existing(&self.root, self.path(), SYMBOL)?;
        (0..file.records()).map(|at| file.read_record(at)).collect()
    }

    fn served(&self) -> u32 {
        generation::served(&self.root, self.path())
            .unwrap()
            .ordinal()
    }

    fn plan(&self) -> Result<Plan, RecoverError> {
        plan(&self.root, self.path(), SYMBOL)
    }

    fn recover(&self) -> Result<Outcome, RecoverError> {
        recover(&self.root, self.path(), SYMBOL)
    }

    /// The importer's re-read of one day, through the doors it takes: the
    /// append door at a month served from the original, and the successor
    /// generation (the repair) at a month served from a revision.
    fn reread(&self, bars: &[Bar]) {
        if self.served() == 0 {
            self.commit(bars);
        } else {
            match repair::repair_month(&self.root, self.path(), SYMBOL, bars, INDEX).unwrap() {
                MonthRepair::NothingNew { .. } | MonthRepair::Repaired { .. } => {}
            }
        }
    }

    /// An IST stamp of this month: day `day`, `minutes` past 09:15.
    fn at(&self, day: i64, minutes: i64) -> i64 {
        self.month.ist_bounds_micros().0 + (day - 1) * DAY + OPEN + minutes * MINUTE
    }

    /// `n` one-minute bars of day `day`, from 09:15 on.
    fn day(&self, day: i64, n: i64) -> Vec<Bar> {
        (0..n).map(|k| bar(self.at(day, k), k)).collect()
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// An invented bar near 12,345 rupees, sane by construction.
fn bar(ts_micros: i64, k: i64) -> Bar {
    let price = 1_234_500 + (k % 50) * 5;
    Bar {
        ts_micros,
        open: price,
        high: price + 40,
        low: price - 40,
        close: price + 10,
        volume: 7 + k % 3,
        open_interest: OI_NULL,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Image {
    bin: Vec<u8>,
    crc: Vec<u8>,
}

/// One write of a day's append, as the bytes it changes.
#[derive(Debug, Clone)]
enum Write {
    /// The bar file's new length (the records past the old end read as
    /// zeros unless their own write is in the state).
    Length(usize),
    /// The records of one block, at their offset in the bar file.
    Records(usize, Vec<u8>),
    /// One checksum entry, at its offset in the sidecar.
    Entry(usize, [u8; 4]),
    /// The header slot, at its offset.
    Slot(usize, Vec<u8>),
}

/// The writes that took `before` to `after`, in the order the writer makes
/// them (records, checksums, slot). The seam: what each write changed.
fn writes(before: &Image, after: &Image) -> Vec<Write> {
    let mut out = Vec::new();
    if after.bin.len() > before.bin.len() {
        out.push(Write::Length(after.bin.len()));
        let mut at = before.bin.len();
        while at < after.bin.len() {
            // TO THE END OF THE BLOCK THIS RECORD IS IN: 73 records of 56.
            let block_end = 32_768 + ((at - 32_768) / 4_088 + 1) * 4_088;
            let end = block_end.min(after.bin.len());
            out.push(Write::Records(at, after.bin[at..end].to_vec()));
            at = end;
        }
    }
    for entry in 0..after.crc.len() / 4 {
        let new = &after.crc[entry * 4..entry * 4 + 4];
        if before.crc.get(entry * 4..entry * 4 + 4) != Some(new) {
            out.push(Write::Entry(entry * 4, new.try_into().unwrap()));
        }
    }
    for slot in [0usize, 16_384] {
        if before.bin[slot..slot + 64] != after.bin[slot..slot + 64] {
            out.push(Write::Slot(slot, after.bin[slot..slot + 64].to_vec()));
        }
    }
    out
}

/// The state a power cut leaves when exactly the writes `mask` names reached
/// the drive. Records past the old end exist only with the length.
fn torn(before: &Image, writes: &[Write], mask: u64) -> Image {
    let chosen = |at: usize| mask & (1 << at) != 0;
    let mut image = before.clone();
    let length = writes
        .iter()
        .enumerate()
        .find_map(|(at, write)| match write {
            Write::Length(len) if chosen(at) => Some(*len),
            _ => None,
        });
    if let Some(len) = length {
        image.bin.resize(len, 0);
    }
    for (at, write) in writes.iter().enumerate() {
        if !chosen(at) {
            continue;
        }
        match write {
            Write::Length(_) => {}
            Write::Records(offset, bytes) => {
                if length.is_some() {
                    image.bin[*offset..*offset + bytes.len()].copy_from_slice(bytes);
                }
            }
            Write::Entry(offset, bytes) => {
                if image.crc.len() < offset + 4 {
                    image.crc.resize(offset + 4, 0);
                }
                image.crc[*offset..*offset + 4].copy_from_slice(bytes);
            }
            Write::Slot(offset, bytes) => {
                image.bin[*offset..*offset + 64].copy_from_slice(bytes);
            }
        }
    }
    image
}

/// The IST day of the month a bar falls on.
fn day_number(store: &Store, ts: i64) -> u8 {
    day_of(store.month, ts)
}

/// RA-276, RA-280, FM-062: EVERY STATE A POWER CUT CAN LEAVE A DAY END IN
/// RECOVERS TO THE LAST GOOD COMMIT AND CONVERGES ON A CLEAN RUN.
///
/// For each subset of a day's writes (records per block, the length, each
/// checksum entry, the slot), the month is rebuilt in that state and:
///
/// * the plan names it clean (the old commit or the new one, nothing torn
///   that a reader meets) or names its cut: the last commit whose records
///   check out, or the whole blocks that do;
/// * a recovery writes the kept records as a NEW file and changes no byte of
///   the original; a second recovery writes nothing;
/// * every bar the cut drops falls on a day the plan names, and the re-read
///   of those days and of the open day converges, bar for bar, on the month a
///   clean run built.
#[test]
fn a_month_cut_at_every_synced_write_recovers_to_its_last_good_commit() {
    let _sink = crate::emits::hold_the_sink();
    // (day one's bars, day two's bars): the old tail block partly full and
    // re-sealed by the day; exactly full, so the day starts a block; and a day
    // reaching three blocks.
    for (first, second) in [(100, 80), (146, 30), (80, 200)] {
        let mut kinds = BTreeMap::new();
        let clean = Store::new(2020, 3);
        clean.commit(&clean.day(2, first));
        let before = clean.image();
        clean.commit(&clean.day(3, second));
        let after = clean.image();
        let whole: Vec<Bar> = clean
            .day(2, first)
            .into_iter()
            .chain(clean.day(3, second))
            .collect();
        let writes = writes(&before, &after);
        assert!(writes.len() >= 4, "length, records, entries, slot");
        for mask in 0..(1u64 << writes.len()) {
            let store = Store::new(2020, 3);
            store.commit(&store.day(2, first));
            store.put(&torn(&before, &writes, mask));
            let original = store.image();
            let reread = match store.plan().unwrap() {
                Plan::Clean { ordinal, n_valid } => {
                    assert_eq!(ordinal, 0);
                    let held = u64::try_from(first).unwrap();
                    assert!(
                        n_valid == held || n_valid == held + u64::try_from(second).unwrap(),
                        "mask {mask}: a clean month is at a commit, held {n_valid}"
                    );
                    *kinds.entry("clean").or_insert(0) += 1;
                    vec![3]
                }
                Plan::Cut {
                    cut,
                    dropped,
                    promoted,
                    refused,
                    ..
                } => {
                    assert!(promoted.is_none());
                    assert!(!refused.is_empty(), "the month's own refusal is named");
                    match cut.basis() {
                        Basis::Commit { generation } => {
                            assert_eq!(generation, 1, "mask {mask}: the day-one commit");
                            assert_eq!(dropped.kept, u64::try_from(first).unwrap());
                            *kinds.entry("commit").or_insert(0) += 1;
                        }
                        Basis::Blocks { blocks, .. } => {
                            assert_eq!(dropped.kept, blocks * 73, "mask {mask}");
                            assert!(dropped.kept < u64::try_from(first).unwrap());
                            *kinds.entry("blocks").or_insert(0) += 1;
                        }
                    }
                    // EVERY DROPPED BAR IS ON A DAY THE PLAN NAMES.
                    let days: Vec<u8> = dropped.days().collect();
                    for dropped_bar in &whole[usize::try_from(dropped.kept).unwrap()
                        ..usize::try_from(dropped.claimed).unwrap()]
                    {
                        assert!(
                            days.contains(&day_number(&store, dropped_bar.ts_micros)),
                            "mask {mask}: a dropped bar falls outside {days:?}"
                        );
                    }
                    let Outcome::Recovered(done) = store.recover().unwrap() else {
                        panic!("mask {mask}: a cut month is recovered");
                    };
                    assert_eq!(done.published, Published::Created);
                    assert!(!done.resumed);
                    assert_eq!(done.record.dropped, dropped);
                    assert_eq!(done.record.source, cut.header());
                    // NOTHING CHANGED IN PLACE: the original's bytes are as the
                    // power cut left them; the kept records are a new file.
                    assert_eq!(store.image(), original, "mask {mask}");
                    assert_eq!(store.served(), 1);
                    assert_eq!(
                        store.served_bars().unwrap(),
                        whole[..usize::try_from(dropped.kept).unwrap()].to_vec()
                    );
                    // IDEMPOTENT: a second run reads the record and writes
                    // nothing anywhere.
                    let tree = store.tree();
                    assert_eq!(store.recover().unwrap(), Outcome::Already(done.record));
                    assert_eq!(store.tree(), tree, "mask {mask}: a rerun writes nothing");
                    days.into_iter().map(i64::from).collect()
                }
                Plan::Recovered(record) => panic!("mask {mask}: never recovered yet: {record:?}"),
            };
            // THE RESUME: the days the plan named and the open day, in order.
            let mut days = reread;
            days.push(3);
            days.sort_unstable();
            days.dedup();
            for day in days {
                let bars = match day {
                    2 => store.day(2, first),
                    3 => store.day(3, second),
                    other => panic!("mask {mask}: day {other} holds no bar of this fixture"),
                };
                store.reread(&bars);
            }
            assert_eq!(
                store.served_bars().unwrap(),
                whole,
                "mask {mask}: the resumed month is the clean month, bar for bar"
            );
        }
        // Each scenario reaches the clean states and at least one cut.
        assert!(kinds.contains_key("clean"), "{first}+{second}: {kinds:?}");
        assert!(kinds.contains_key("commit"), "{first}+{second}: {kinds:?}");
        if first % 73 != 0 {
            assert!(
                kinds.contains_key("blocks"),
                "{first}+{second}: a re-sealed tail entry whose records never landed: {kinds:?}"
            );
        }
    }
}

/// FM-062's own test: A POWER CUT INSIDE A BATCH IS NAMED AND REPAIRED.
///
/// The month's newest slot reached the platter and its records and checksums
/// did not (the drive's ordering, simulated byte for byte). After the
/// restart the month is refused by name; the door cuts it at its last good
/// commit, and the repaired generation holds every bar the month held before
/// the batch. The day re-read then lands.
#[test]
fn a_power_cut_inside_a_batch_is_named_and_repaired() {
    let _sink = crate::emits::hold_the_sink();
    for length_reached_the_disk in [false, true] {
        let store = Store::new(2020, 3);
        store.commit(&store.day(2, 100));
        let before = store.image();
        store.commit(&store.day(3, 80));
        let after = store.image();
        let writes = writes(&before, &after);
        // THE SLOT, AND THE LENGTH WHEN ASKED: no record, no checksum entry.
        let mask = writes
            .iter()
            .enumerate()
            .filter(|(_, write)| {
                matches!(write, Write::Slot(..))
                    || (length_reached_the_disk && matches!(write, Write::Length(_)))
            })
            .fold(0u64, |mask, (at, _)| mask | (1 << at));
        store.put(&torn(&before, &writes, mask));
        let original = store.image();

        // THE NAMED REFUSAL: every reader is refused, never served short.
        let refusal = store.served_bars().expect_err("the month is refused");
        if length_reached_the_disk {
            assert!(
                matches!(refusal, StoreError::BlockChecksum { block: 1, .. }),
                "{refusal:?}"
            );
        } else {
            assert!(
                matches!(
                    refusal,
                    StoreError::Format {
                        source: crate::format::FormatError::CounterExceedsFile,
                        ..
                    }
                ),
                "{refusal:?}"
            );
        }

        let Plan::Cut { cut, dropped, .. } = store.plan().unwrap() else {
            panic!("a torn month has a cut");
        };
        assert_eq!(cut.basis(), Basis::Commit { generation: 1 });
        assert_eq!(
            dropped,
            Dropped {
                kept: 100,
                last_kept_ts: store.at(2, 99),
                claimed: 180,
                claimed_generation: 2,
                claimed_last_ts: store.at(3, 79),
                bars: 80,
                first_day: 2,
                last_day: 3,
            }
        );
        let Outcome::Recovered(done) = store.recover().unwrap() else {
            panic!("recovered");
        };
        assert_eq!(done.record.revision, Revision::new(1).unwrap());
        // THE REPAIRED GENERATION HOLDS EVERY EARLIER BAR.
        assert_eq!(store.served_bars().unwrap(), store.day(2, 100));
        assert_eq!(store.image(), original, "the original is never touched");
        assert_eq!(
            read_record(&store.root, store.path()).unwrap(),
            Some(done.record)
        );
        // The day then resumes, and the month is the clean month.
        store.reread(&store.day(3, 80));
        let whole: Vec<Bar> = store
            .day(2, 100)
            .into_iter()
            .chain(store.day(3, 80))
            .collect();
        assert_eq!(store.served_bars().unwrap(), whole);
    }
}

/// EMPTY: a month whose only commit that checks out holds nothing (born in
/// the open batch) has nothing safe to keep. It is refused by name, and no
/// record, revision or directory is made.
#[test]
fn a_month_that_held_nothing_proven_is_refused_by_name_and_left_as_it_was() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    // The month exists at its genesis, and its first commit tears.
    drop(BarFile::open_or_create(&store.root, store.path(), SYMBOL).unwrap());
    let before = store.image();
    store.commit(&store.day(2, 50));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot_only = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .map(|at| 1u64 << at)
        .unwrap();
    store.put(&torn(&before, &writes, slot_only));
    let tree = store.tree();
    let refused = store.plan().expect_err("nothing verifies");
    assert_eq!(refused.word(), "nothing_verifies");
    let RecoverError::NoCommit { tried, .. } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(tried.len(), 2, "both slots were tried: {tried:?}");
    assert!(refused.to_string().contains("restore the month"));
    assert_eq!(store.recover().unwrap_err(), refused);
    assert_eq!(store.tree(), tree, "nothing was written");
    assert!(!store.root.join(super::ROOT).exists());
    assert!(!store.root.join("bar-revisions-v1").exists());
}

/// ONE RECORD: a month that held one bar before the batch keeps that bar.
#[test]
fn a_one_bar_month_recovers_its_one_bar() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 1));
    let before = store.image();
    store.commit(&store.day(3, 5));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let Outcome::Recovered(done) = store.recover().unwrap() else {
        panic!("recovered");
    };
    assert_eq!(done.record.dropped.kept, 1);
    assert_eq!(done.record.dropped.bars, 5);
    assert_eq!(store.served_bars().unwrap(), store.day(2, 1));
}

/// MAXIMUM, AND A DOOR CUT OFF MID-WAY (a combination): a one-minute month
/// at its row ceiling (every minute of 31 days, 44,640 bars) whose last day
/// tore, recovered by a door that was cut off after its publication and run
/// again. The kept bars stay inside the ceiling, the second run reuses the
/// complete revision rather than writing another, and the re-read lands the
/// month at the ceiling exactly.
#[test]
fn a_full_month_cut_while_its_door_was_cut_off_resumes_at_the_row_ceiling() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    let start = store.month.ist_bounds_micros().0;
    let every: Vec<Bar> = (0..44_640)
        .map(|minute| bar(start + minute * MINUTE, minute))
        .collect();
    assert_eq!(
        repair::row_ceiling(store.path()),
        44_640,
        "the month's ceiling"
    );
    store.commit(&every[..43_200]);
    let before = store.image();
    store.commit(&every[43_200..]);
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let Plan::Cut { cut, dropped, .. } = store.plan().unwrap() else {
        panic!("cut");
    };
    assert_eq!((dropped.kept, dropped.claimed), (43_200, 44_640));
    assert_eq!((dropped.first_day, dropped.last_day), (30, 31));
    // THE FIRST DOOR PUBLISHED AND WAS CUT OFF BEFORE ITS PROMOTION.
    let kept = &every[..43_200];
    let first = within(&cut, || {
        repair::publish(
            &store.root,
            store.path(),
            SYMBOL,
            Revision::new(1).unwrap(),
            cut.header(),
            kept,
        )
    })
    .unwrap();
    assert_eq!(first, Published::Created);
    assert_eq!(store.served(), 0, "published, not served");
    let Outcome::Recovered(done) = store.recover().unwrap() else {
        panic!("recovered");
    };
    assert_eq!(done.published, Published::Reused, "the complete revision");
    assert_eq!(done.record.revision, Revision::new(1).unwrap());
    assert_eq!(store.served_bars().unwrap(), kept.to_vec());
    store.reread(&every[43_200..]);
    assert_eq!(store.served_bars().unwrap().len(), 44_640);
}

/// THE MONTH AND YEAR EDGES: a December whose first bar is at 00:00 IST on
/// the 1st and whose last is at 23:59 IST on the 31st names days 1 and 31,
/// and the record keeps them across the year end.
#[test]
fn the_month_and_year_edges_name_the_first_and_last_days() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2019, 12);
    let (from, until) = store.month.ist_bounds_micros();
    assert_eq!(day_of(store.month, from), 1);
    assert_eq!(day_of(store.month, until - MINUTE), 31);
    assert_eq!(day_of(store.month, from - 1), 1, "clamped, never day 0");
    assert_eq!(
        day_of(store.month, until + DAY),
        31,
        "clamped, never day 32"
    );
    store.commit(&[bar(from, 0)]);
    let before = store.image();
    store.commit(&[bar(until - MINUTE, 1)]);
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let Outcome::Recovered(done) = store.recover().unwrap() else {
        panic!("recovered");
    };
    assert_eq!(
        (done.record.dropped.first_day, done.record.dropped.last_day),
        (1, 31)
    );
    assert_eq!(done.record.dropped.days().count(), 31);
    assert_eq!(done.record.dropped.claimed_last_ts, until - MINUTE);
    let read = read_record(&store.root, store.path()).unwrap().unwrap();
    assert_eq!(read, done.record);
}

/// TWO WRITERS: a writer holding the month refuses the door by name and
/// nothing is written; two doors at once refuse the second by the record
/// file's lock.
#[test]
fn a_writer_holding_the_month_refuses_the_door_and_nothing_is_written() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let before = store.image();
    store.commit(&store.day(3, 80));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let tree = store.tree();
    {
        // A WRITER'S EXCLUSIVE LOCK, held as the writer door holds it.
        let lock = fs::File::open(store.file(FileKind::Lock)).unwrap();
        let _held = crate::flock::Flock::try_lock(lock, store.file(FileKind::Lock)).unwrap();
        let busy = store.plan().unwrap_err();
        assert_eq!(busy.word(), "month_busy");
        assert_eq!(store.recover().unwrap_err(), busy);
        assert_eq!(store.tree(), tree);
    }
    {
        // ANOTHER DOOR HOLDS THE RECORD FILE.
        let records = super::RecordFile::open(&store.root, store.path()).unwrap();
        let busy = store.recover().unwrap_err();
        assert!(
            matches!(busy, RecoverError::Store(StoreError::Locked { .. })),
            "{busy:?}"
        );
        records.release().unwrap();
        assert_eq!(store.served(), 0, "nothing was published");
    }
    assert!(matches!(store.recover().unwrap(), Outcome::Recovered(_)));
}

/// CRASH AND RESUME: a door cut off between its promotion and its record
/// finds its own promotion and writes only the record it owes.
#[test]
fn a_door_cut_off_after_its_promotion_writes_the_record_it_owes() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let before = store.image();
    store.commit(&store.day(3, 80));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let Plan::Cut { cut, .. } = store.plan().unwrap() else {
        panic!("cut");
    };
    let revision = Revision::new(1).unwrap();
    let kept = store.day(2, 100);
    within(&cut, || {
        repair::publish(
            &store.root,
            store.path(),
            SYMBOL,
            revision,
            cut.header(),
            &kept,
        )
        .unwrap();
        generation::promote(&store.root, store.path(), SYMBOL, revision).unwrap();
    });
    assert_eq!(store.served(), 1);
    let Plan::Cut { promoted, .. } = store.plan().unwrap() else {
        panic!("the unrecorded promotion is this door's");
    };
    assert_eq!(promoted.map(|pointer| pointer.revision), Some(revision));
    let tree = store.tree();
    let Outcome::Recovered(done) = store.recover().unwrap() else {
        panic!("resumed");
    };
    assert!(done.resumed);
    assert_eq!(done.record.record, 0);
    let mut grew: Vec<PathBuf> = store
        .tree()
        .into_iter()
        .filter(|(name, bytes)| tree.get(name) != Some(bytes))
        .map(|(name, _)| name)
        .collect();
    grew.sort();
    assert_eq!(
        grew,
        vec![
            record_path(&store.root, store.path())
                .strip_prefix(&store.root)
                .unwrap()
                .to_path_buf()
        ],
        "only the record was written"
    );
    assert!(matches!(store.recover().unwrap(), Outcome::Already(_)));
}

/// TORN AND GARBAGE RECORD FILES: a torn tail after a whole record is
/// stepped over; a file of which no record verifies is refused, never read
/// as "not recovered"; a record moved to another position does not verify.
#[test]
fn a_torn_record_file_is_stepped_over_and_garbage_is_refused() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    let record = Record {
        revision: Revision::new(3).unwrap(),
        basis: Basis::Blocks {
            blocks: 2,
            generation: 4,
        },
        source: Header {
            generation: 4,
            n_valid: 146,
            first_ts_micros: store.at(2, 0),
            last_ts_micros: store.at(2, 145),
            ..Header::genesis(SYMBOL, 60, crate::format::FLAG_CHECKSUMS)
        },
        dropped: Dropped {
            kept: 146,
            last_kept_ts: store.at(2, 145),
            claimed: 200,
            claimed_generation: 5,
            claimed_last_ts: store.at(3, 53),
            bars: 54,
            first_day: 2,
            last_day: 3,
        },
        bin_len: 43_968,
        region_crc: 0xDEAD_BEEF,
        recovered_at_micros: 1_790_000_000_000_000,
        reread_named: true,
        record: 0,
    };
    let bytes = record.encode().unwrap();
    assert_eq!(bytes.len() as u64, RECORD_LEN);
    assert_eq!(super::Record::decode(&bytes, 0), Some(record));
    assert_eq!(super::Record::decode(&bytes, 1), None, "moved");
    for at in [0usize, 8, 10, 11, 16, 24, 40, 104, 144, 200, 252] {
        let mut damaged = bytes;
        damaged[at] ^= 0x04;
        assert_eq!(super::Record::decode(&damaged, 0), None, "byte {at}");
    }
    let mut unknown_basis = bytes;
    unknown_basis[10] = 9;
    let crc = super::seal(&unknown_basis, 0);
    unknown_basis[252..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(super::Record::decode(&unknown_basis, 0), None, "basis 9");
    let mut unknown_flag = bytes;
    unknown_flag[13] = 2;
    let crc = super::seal(&unknown_flag, 0);
    unknown_flag[252..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(super::Record::decode(&unknown_flag, 0), None, "flag 2");
    let mut unflagged = bytes;
    unflagged[13] = 0;
    let crc = super::seal(&unflagged, 0);
    unflagged[252..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        super::Record::decode(&unflagged, 0),
        Some(Record {
            reread_named: false,
            ..record
        })
    );

    let at = record_path(&store.root, store.path());
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    assert_eq!(read_record(&store.root, store.path()).unwrap(), None);
    fs::write(&at, b"").unwrap();
    assert_eq!(read_record(&store.root, store.path()).unwrap(), None);
    let mut torn = bytes.to_vec();
    torn.extend_from_slice(&[0xAB; 100]);
    fs::write(&at, &torn).unwrap();
    assert_eq!(
        read_record(&store.root, store.path()).unwrap(),
        Some(record)
    );
    fs::write(&at, [0x5A; 300]).unwrap();
    let refused = read_record(&store.root, store.path()).unwrap_err();
    assert_eq!(refused.word(), "record_refused");
    assert!(refused.to_string().contains("none of its 1 whole record"));
    fs::write(&at, vec![0u8; usize::try_from(super::MAX_LEN).unwrap() + 1]).unwrap();
    assert!(
        read_record(&store.root, store.path())
            .unwrap_err()
            .to_string()
            .contains("past the")
    );
    fs::remove_file(&at).unwrap();
    fs::create_dir(&at).unwrap();
    assert!(read_record(&store.root, store.path()).is_err());
}

/// NOT THIS DOOR'S: a month held sealed, one with an overlay or a greeks
/// sibling, and an unsealed month (no checksum proves any record) are each
/// refused by name with nothing written.
#[test]
fn a_sealed_an_overlaid_or_an_unsealed_month_is_not_this_doors() {
    let _sink = crate::emits::hold_the_sink();
    for (kind, word) in [
        (FileKind::SealedBars, "not_raw"),
        (FileKind::Overlay, "not_raw"),
        (FileKind::Greeks, "not_raw"),
    ] {
        let store = Store::new(2020, 3);
        store.commit(&store.day(2, 100));
        fs::write(store.file(kind), b"invented").unwrap();
        let tree = store.tree();
        let refused = store.recover().unwrap_err();
        assert_eq!(refused.word(), word, "{kind:?}");
        assert!(refused.to_string().contains("nothing was written"));
        assert_eq!(store.tree(), tree);
    }
    // UNSEALED: a version-2 month born without the checksum flag.
    let store = Store::new(2020, 3);
    let bars = store.day(2, 3);
    let header = Header {
        format_version: 2,
        flags: 0,
        generation: 1,
        n_valid: 3,
        first_ts_micros: bars[0].ts_micros,
        last_ts_micros: bars[2].ts_micros,
        ..Header::genesis(SYMBOL, 60, crate::format::FLAG_CHECKSUMS)
    };
    let claim = Header {
        generation: 2,
        n_valid: 9,
        ..header
    };
    let mut bin = vec![0u8; 32_768];
    for slot in [header.commit().unwrap(), claim.commit().unwrap()] {
        let offset = usize::try_from(slot.offset).unwrap();
        bin[offset..offset + 64].copy_from_slice(&slot.bytes);
    }
    for one in &bars {
        bin.extend_from_slice(&one.image());
    }
    let bars_at = store.file(FileKind::Bars);
    fs::create_dir_all(bars_at.parent().unwrap()).unwrap();
    fs::write(&bars_at, &bin).unwrap();
    fs::write(store.file(FileKind::Lock), b"").unwrap();
    let refused = store.plan().unwrap_err();
    assert_eq!(refused.word(), "unsealed");
    assert!(refused.to_string().contains("no block checksums"));
}

/// DAMAGE INSIDE THE MONTH'S HISTORY is not a torn tail: a flipped bit in
/// the first block, with whole blocks after it that verify, is refused, and
/// no cut drops the verified history after it.
#[test]
fn damage_inside_the_months_history_is_not_a_torn_tail() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 150));
    store.commit(&store.day(3, 150));
    let mut image = store.image();
    image.bin[32_768 + 9] ^= 0x01;
    store.put(&image);
    let tree = store.tree();
    let refused = store.recover().unwrap_err();
    let RecoverError::NotTheTail {
        block,
        verified_after,
        ..
    } = refused
    else {
        panic!("{refused:?}");
    };
    assert_eq!(block, 0);
    assert_eq!(verified_after, 3, "blocks 1, 2 and 3 of 300 bars verify");
    assert!(refused.to_string().contains("not the torn tail"));
    assert_eq!(store.tree(), tree);
}

/// TWO COMMITS IN ONE BATCH AT THE YEAR END (a combination): the month's
/// two newest commits were both in the open batch, so no slot holds the
/// last durable commit, and the power cut lost the records of both. The
/// whole blocks that verify are the cut, the day the cut reaches into is
/// named as dropped, and the re-read of the three days converges.
#[test]
fn two_commits_torn_in_one_batch_at_a_year_end_keep_the_whole_blocks() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2019, 12);
    store.commit(&store.day(29, 100));
    let durable = store.image();
    store.commit(&store.day(30, 60));
    store.commit(&store.day(31, 60));
    let mut image = store.image();
    // NO RECORD OF EITHER COMMIT REACHED THE DISK; everything else did.
    let from = durable.bin.len();
    image.bin[from..].fill(0);
    store.put(&image);
    let Plan::Cut {
        cut,
        dropped,
        tried,
        ..
    } = store.plan().unwrap()
    else {
        panic!("cut");
    };
    assert_eq!(tried.len(), 2, "both slots tried: {tried:?}");
    assert_eq!(
        cut.basis(),
        Basis::Blocks {
            blocks: 1,
            generation: 2
        }
    );
    assert_eq!(
        (dropped.kept, dropped.claimed, dropped.bars),
        (73, 220, 147)
    );
    assert_eq!((dropped.first_day, dropped.last_day), (29, 31));
    let Outcome::Recovered(done) = store.recover().unwrap() else {
        panic!("recovered");
    };
    assert_eq!(done.record.basis, cut.basis());
    for day in 29..=31 {
        let n = if day == 29 { 100 } else { 60 };
        store.reread(&store.day(day, n));
    }
    let whole: Vec<Bar> = store
        .day(29, 100)
        .into_iter()
        .chain(store.day(30, 60))
        .chain(store.day(31, 60))
        .collect();
    assert_eq!(store.served_bars().unwrap(), whole);
}

/// OUT OF THE BOX: A CUT NEVER APPLIES TO BYTES IT WAS NOT PLANNED AGAINST,
/// NOR ON ANOTHER THREAD, NOR TO ANOTHER FILE. Every other reader keeps the
/// month's refusal while the door holds its cut; a byte of the header region
/// no reader decodes, or one more byte at the file's end, moves the bytes and
/// the cut no longer opens the month.
#[test]
fn a_cut_never_applies_to_other_bytes_another_thread_or_another_file() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let before = store.image();
    store.commit(&store.day(3, 80));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let Plan::Cut { cut, .. } = store.plan().unwrap() else {
        panic!("cut");
    };
    let bars = store.file(FileKind::Bars);
    assert_eq!(scoped(&bars), None, "outside the door's scope");
    within(&cut, || {
        assert_eq!(scoped(&bars), Some(cut.clone()));
        assert_eq!(scoped(&store.file(FileKind::Checksums)), None);
        assert_eq!(store.served_bars().unwrap().len(), 100, "this thread");
        std::thread::scope(|threads| {
            threads.spawn(|| {
                assert_eq!(scoped(&bars), None);
                assert!(store.served_bars().is_err(), "another thread is refused");
            });
        });
        // NESTED, the outer cut comes back when the inner one ends.
        let other = Cut {
            bars: PathBuf::from("/nowhere.bin"),
            ..cut.clone()
        };
        within(&other, || assert_eq!(scoped(&bars), None));
        assert_eq!(scoped(&bars), Some(cut.clone()));
    });
    assert_eq!(scoped(&bars), None, "the scope ends with the door");
    // A RESTORE ON UNWIND: a step that panics leaves no cut behind.
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        within(&cut, || panic!("a step that dies"));
    }));
    assert!(unwound.is_err());
    assert_eq!(scoped(&bars), None);

    let torn_image = store.image();
    let mut reserved = torn_image.clone();
    reserved.bin[1_000] ^= 0x01;
    store.put(&reserved);
    assert!(store.served_bars().is_err());
    assert!(!cut.fits(
        u64::try_from(reserved.bin.len()).unwrap(),
        &reserved.bin[..32_768]
    ));
    within(&cut, || {
        assert!(store.served_bars().is_err(), "moved region");
    });
    let mut longer = torn_image;
    longer.bin.push(0);
    store.put(&longer);
    within(&cut, || {
        assert!(store.served_bars().is_err(), "moved length");
    });
    assert_eq!(cut.bin_len(), u64::try_from(longer.bin.len()).unwrap() - 1);
    assert_ne!(cut.region_crc(), 0);
}

/// OUT OF THE BOX: A RECORD DIRECTORY LINKED ELSEWHERE is refused before
/// anything is published, so a shadow's link never carries a write.
#[test]
fn a_linked_record_directory_is_refused_before_anything_is_published() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let before = store.image();
    store.commit(&store.day(3, 80));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let elsewhere = store.root.join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, store.root.join(super::ROOT)).unwrap();
    let refused = store.recover().unwrap_err();
    assert!(
        matches!(refused, RecoverError::Store(StoreError::Symlinked { .. })),
        "{refused:?}"
    );
    assert_eq!(store.served(), 0);
    assert!(!store.root.join("bar-revisions-v1").exists());
    assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
}

/// CLEAN AND ALREADY REPAIRED: a month that reads clean, and one served from
/// a repair revision this door did not make, are answered as they are and
/// left alone; a damaged revision this door did not make is not its own.
#[test]
fn clean_and_repaired_months_are_answered_and_a_foreign_revision_is_refused() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let tree = store.tree();
    assert_eq!(
        store.plan().unwrap(),
        Plan::Clean {
            ordinal: 0,
            n_valid: 100
        }
    );
    assert_eq!(
        store.recover().unwrap(),
        Outcome::Clean {
            ordinal: 0,
            n_valid: 100
        }
    );
    assert_eq!(store.tree(), tree, "a clean month writes nothing");
    // A day-five hole filled by a repair: served from revision 1.
    store.commit(&store.day(9, 10));
    repair::repair_month(&store.root, store.path(), SYMBOL, &store.day(5, 10), INDEX).unwrap();
    assert_eq!(store.served(), 1);
    assert_eq!(
        store.plan().unwrap(),
        Plan::Clean {
            ordinal: 1,
            n_valid: 120
        }
    );
    // The revision's own bar file damaged after its promotion.
    let revision_bars = store.path().to_path_buf(&repair::revision_root(
        &store.root,
        Revision::new(1).unwrap(),
    ));
    let mut bytes = fs::read(&revision_bars).unwrap();
    bytes[32_768 + 3] ^= 0x02;
    fs::write(&revision_bars, &bytes).unwrap();
    let refused = store.plan().unwrap_err();
    assert_eq!(refused.word(), "not_raw");
    assert!(refused.to_string().contains("did not promote"));
}

/// GARBAGE PARAMETERS AND A MISSING MONTH: a path that is not the bars, a
/// month that is not there and a month with no lock beside it are refused by
/// name, and nothing is created.
#[test]
fn a_wrong_path_a_missing_month_and_a_lockless_month_are_refused_by_name() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    let wrong = plan(&store.root, store.path_of(FileKind::Checksums), SYMBOL).unwrap_err();
    assert!(
        matches!(wrong, RecoverError::Store(StoreError::NotABarPath { .. })),
        "{wrong:?}"
    );
    assert_eq!(wrong.word(), "store_refused");
    let missing = store.recover().unwrap_err();
    assert_eq!(missing.word(), "month_missing");
    assert_eq!(
        fs::read_dir(&store.root).unwrap().count(),
        0,
        "nothing made"
    );
    store.commit(&store.day(2, 10));
    fs::remove_file(store.file(FileKind::Lock)).unwrap();
    let lockless = store.plan().unwrap_err();
    assert_eq!(lockless.word(), "io");
    assert!(lockless.to_string().contains("open the month's lock"));
}

/// EVERY REFUSAL SAYS WHAT IT IS, and every basis what it kept.
#[test]
fn every_refusal_and_basis_is_named() {
    let path = PathBuf::from("/store/bars/x.bin");
    let tried = vec![super::Tried {
        generation: 2,
        n_valid: 9,
        refused: "block 0".to_owned(),
    }];
    let cases = [
        (
            RecoverError::Repair(repair::RepairError::StaleSource),
            "repair_refused",
            "merge again",
        ),
        (
            RecoverError::NoCommit {
                refused: "torn".to_owned(),
                tried,
            },
            "nothing_verifies",
            "generation 2 (9 bars): block 0",
        ),
        (
            RecoverError::Record {
                path: path.clone(),
                reason: "why".to_owned(),
            },
            "record_refused",
            "x.bin: why",
        ),
        (
            RecoverError::Io {
                path,
                operation: "sync",
                kind: std::io::ErrorKind::Other,
            },
            "io",
            "sync /store/bars/x.bin: Other",
        ),
    ];
    for (error, word, says) in cases {
        assert_eq!(error.word(), word);
        assert!(error.to_string().contains(says), "{error}");
        assert!(error.to_string().starts_with(word), "{error}");
    }
    let commit = Basis::Commit { generation: 7 };
    let blocks = Basis::Blocks {
        blocks: 3,
        generation: 6,
    };
    assert_eq!((commit.as_str(), commit.generation()), ("commit", 7));
    assert_eq!((blocks.as_str(), blocks.generation()), ("blocks", 6));
    assert_eq!(commit.to_string(), "the commit of generation 7");
    assert!(blocks.to_string().starts_with("the first 3 whole block(s)"));
    let _: &dyn std::error::Error = &RecoverError::Unsealed {
        path: PathBuf::new(),
    };
    assert_eq!(
        RecoverError::from(StoreError::EmptyBatch),
        RecoverError::Store(StoreError::EmptyBatch)
    );
}

/// The paths the door names are the month's own, under its own root.
#[test]
fn the_record_lives_beside_the_month_under_its_own_root() {
    let store = Store::new(2020, 3);
    let at = record_path(Path::new("/s"), store.path());
    assert_eq!(
        at,
        PathBuf::from("/s/bar-recoveries-v1/bars/gdfl/NSE/INDEX/FIXTURE/1min/2020-03.recover-v1")
    );
    assert!(matches!(
        generation::served(&store.root, store.path()),
        Ok(Served::Original)
    ));
}

/// THE RE-READ IS NOTED ONCE, AGAINST ITS OWN RECOVERY: the copy with the
/// flag is appended once, a second note writes nothing, a later run of the
/// door answers with the flagged record, and a note against a recovery that
/// is not the month's last is refused.
#[test]
fn the_re_read_is_noted_once_against_its_own_recovery() {
    let _sink = crate::emits::hold_the_sink();
    let store = Store::new(2020, 3);
    store.commit(&store.day(2, 100));
    let before = store.image();
    store.commit(&store.day(3, 80));
    let after = store.image();
    let writes = writes(&before, &after);
    let slot = writes
        .iter()
        .position(|write| matches!(write, Write::Slot(..)))
        .unwrap();
    store.put(&torn(&before, &writes, 1 << slot));
    let foreign = Record {
        revision: Revision::new(9).unwrap(),
        ..read_record_or_recover(&store)
    };
    let Outcome::Already(recorded) = store.recover().unwrap() else {
        panic!("already");
    };
    assert!(!recorded.reread_named);
    let refused = super::note_reread(&store.root, store.path(), &foreign).unwrap_err();
    assert_eq!(refused.word(), "record_refused");
    let noted = super::note_reread(&store.root, store.path(), &recorded).unwrap();
    assert!(noted.reread_named);
    assert_eq!(noted.record, 1, "a copy, appended");
    let tree = store.tree();
    assert_eq!(
        super::note_reread(&store.root, store.path(), &recorded).unwrap(),
        noted
    );
    assert_eq!(store.tree(), tree, "a second note writes nothing");
    assert_eq!(store.recover().unwrap(), Outcome::Already(noted));
    assert_eq!(store.plan().unwrap(), Plan::Recovered(noted));
}

/// The record a first recovery of `store` writes.
fn read_record_or_recover(store: &Store) -> Record {
    match store.recover().unwrap() {
        Outcome::Recovered(done) => done.record,
        other => panic!("{other:?}"),
    }
}
