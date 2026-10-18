//! The read door over both forms of a month (SC-3, D-4965; design §4.4, §6).
//!
//! Every test writes a raw month through the writer door into its own temp
//! root, seals it with [`crate::sealed::build`] (the sealer is SC-6's; these
//! tests only need its bytes), and asks the read door. Prices are invented and
//! absurd on purpose (tens of millions of rupees), so no row can be mistaken
//! for a vendor's.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;

use super::{BarFile, Form, Reindexed, StoreError, TimeLookup};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 0x5C3;
const MICROS: i64 = 1_000_000;

thread_local! {
    /// What runs between the door's first (`.sbar`) and second (`.bin`) open.
    static BETWEEN: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

/// Called by [`BarFile::open_existing`] between its first and second opens.
pub(super) fn between_opens() {
    if let Some(work) = BETWEEN.with(|hook| hook.borrow_mut().take()) {
        work();
    }
}

fn arm_between_opens(work: impl FnOnce() + 'static) {
    BETWEEN.with(|hook| *hook.borrow_mut() = Some(Box::new(work)));
}

pub(super) struct Root(pub(super) PathBuf);

impl Root {
    pub(super) fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-sealed-door-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, path: StorePath<'_>, kind: FileKind) -> PathBuf {
        path.with_file(kind).to_path_buf(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn month_path(
    contract: Option<brutex_core::instrument::Contract>,
) -> StorePath<'static> {
    path_at(
        contract,
        Timeframe::SECOND_1,
        YearMonth::new(2031, 5).unwrap(),
    )
}

pub(super) fn path_at(
    contract: Option<brutex_core::instrument::Contract>,
    timeframe: Timeframe,
    month: YearMonth,
) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: if contract.is_some() { "FNO" } else { "INDEX" },
        symbol: "NIFTY",
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `count` one-second bars from 09:15:00 IST on Monday 5 May 2031; every
/// fifth has a real open interest, the rest the null, so both kinds cross
/// every frame.
pub(super) fn seconds(count: u64) -> Vec<Bar> {
    let open =
        YearMonth::new(2031, 5).unwrap().ist_bounds_micros().0 + (4 * 86_400 + 555 * 60) * MICROS;
    (0..i64::try_from(count).unwrap())
        .map(|k| Bar {
            ts_micros: open + k * MICROS,
            open: 2_100_000_000 + k % 300,
            high: 2_100_000_500,
            low: 2_099_999_500,
            close: 2_100_000_000 + (k * 7) % 400,
            volume: 1 + k % 13,
            open_interest: if k % 5 == 0 { 9_000_000 + k } else { OI_NULL },
        })
        .collect()
}

/// A raw month of `bars` under `root`, through the writer door.
pub(super) fn write_raw(root: &Path, path: StorePath<'_>, bars: &[Bar]) {
    let mut writer = BarFile::open_or_create(root, path, SYMBOL).unwrap();
    if !bars.is_empty() {
        writer.append(bars).unwrap();
    }
}

/// The month's `.sbar` bytes, built from what the raw door reads.
pub(super) fn sealed_bytes(root: &Path, path: StorePath<'_>) -> Vec<u8> {
    let raw = BarFile::open_existing(root, path, SYMBOL).unwrap();
    assert_eq!(raw.form(), Form::Raw);
    let bars = raw.read_range(0, raw.records()).unwrap();
    let prv = fs::read(path.with_file(FileKind::Provenance).to_path_buf(root)).ok();
    crate::sealed::build(path, &raw.header(), &bars, prv.as_deref(), None)
        .unwrap()
        .to_bytes()
}

/// Seals the month beside its raw form, as the sealer's rename would.
pub(super) fn seal(root: &Path, path: StorePath<'_>) {
    let bytes = sealed_bytes(root, path);
    let at = path.with_file(FileKind::SealedBars).to_path_buf(root);
    let staged = at.with_extension("sbar-staged");
    fs::write(&staged, bytes).unwrap();
    fs::rename(&staged, &at).unwrap();
}

/// The month as the raw form answers it: every row, and the lookup of every
/// stamp from one before the first to one after the last.
fn answers(file: &BarFile, bars: &[Bar]) -> (Vec<Bar>, Vec<u64>) {
    let rows = (0..file.records())
        .map(|i| file.read_record(i).unwrap())
        .collect();
    let first = bars.first().map_or(0, |bar| bar.ts_micros);
    let last = bars.last().map_or(0, |bar| bar.ts_micros);
    let step = ((last - first) / 97).max(MICROS / 2);
    let mut probes = vec![first - 1, first, last, last + 1, i64::MIN, i64::MAX];
    let mut ts = first;
    while ts <= last {
        probes.push(ts);
        probes.push(ts + 1);
        ts += step;
    }
    let lookups = probes
        .iter()
        .map(|&ts| file.first_at_or_after(ts).unwrap())
        .collect();
    (rows, lookups)
}

#[test]
fn every_size_reads_the_same_rows_and_finds_the_same_stamps_in_both_forms() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // Empty, one, a frame's edges, the small-month threshold's edges, and past
    // 64 frames, where the sealed month carries a time table.
    for n in [0, 1, 255, 256, 257, 1_023, 1_024, 1_025, 16_385, 20_000] {
        let root = Root::new(&format!("sizes-{n}"));
        let path = month_path(None);
        let bars = seconds(n);
        write_raw(&root.0, path, &bars);
        let raw = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
        let (rows, lookups) = answers(&raw, &bars);
        assert_eq!(rows, bars, "n={n}: the raw form holds what was written");
        let header = raw.header();
        drop(raw);

        seal(&root.0, path);
        let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
        assert_eq!(sealed.form(), Form::Sealed, "n={n}");
        assert_eq!(sealed.time_lookup(), TimeLookup::Sealed, "n={n}");
        assert_eq!(sealed.header(), header, "n={n}: the head copies the header");
        assert_eq!(
            sealed.path(),
            root.file(path, FileKind::SealedBars),
            "n={n}: the handle names the file it reads"
        );
        assert_eq!(answers(&sealed, &bars), (rows, lookups), "n={n}");
        assert_eq!(sealed.read_range(0, n).unwrap(), bars, "n={n}");
        if n > 3 {
            let mid = n / 2;
            assert_eq!(
                sealed.read_range(mid - 1, 3).unwrap(),
                bars[usize::try_from(mid - 1).unwrap()..usize::try_from(mid + 2).unwrap()],
                "n={n}: a page across the middle"
            );
            let scattered = [n - 1, 0, mid, n - 1, 1, mid];
            let want: Vec<Bar> = scattered
                .iter()
                .map(|&i| bars[usize::try_from(i).unwrap()])
                .collect();
            assert_eq!(sealed.read_rows_at(&scattered).unwrap(), want, "n={n}");
        }
        assert!(
            matches!(
                sealed.read_record(n),
                Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
            ),
            "n={n}: past the commit is refused, as for the raw form"
        );
        assert!(matches!(
            sealed.read_range(n, 1),
            Err(StoreError::NotCommitted { .. })
        ));
        assert!(matches!(
            sealed.read_rows_at(&[n]),
            Err(StoreError::NotCommitted { .. })
        ));
        assert_eq!(sealed.sealed_head().map(|head| head.n_valid), Some(n));
    }
}

#[test]
fn a_raw_month_answers_the_page_reads_through_its_own_cache() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("raw-pages");
    let path = month_path(None);
    let bars = seconds(300);
    write_raw(&root.0, path, &bars);
    let raw = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(raw.form(), Form::Raw);
    assert_eq!(raw.form().as_str(), "raw");
    assert_eq!(Form::Sealed.as_str(), "sealed");
    assert!(raw.sealed_head().is_none());
    assert_eq!(raw.read_range(10, 100).unwrap(), bars[10..110]);
    assert_eq!(
        raw.read_rows_at(&[299, 0, 150]).unwrap(),
        vec![bars[299], bars[0], bars[150]]
    );
    assert!(matches!(
        raw.read_range(250, 51),
        Err(StoreError::NotCommitted { .. })
    ));
    assert!(matches!(
        raw.read_range(u64::MAX, 2),
        Err(StoreError::NotCommitted { .. })
    ));
}

#[test]
fn a_sealed_month_alone_is_found_without_its_raw_form() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // After a retirement (SC-9) only the sealed form remains: no `.bin`, no
    // `.lock`, no `.crc`. The door opens it and takes no lock.
    let root = Root::new("alone");
    let path = month_path(None);
    let bars = seconds(700);
    write_raw(&root.0, path, &bars);
    let bytes = sealed_bytes(&root.0, path);
    let other = Root::new("alone-copy");
    let at = root.file(path, FileKind::SealedBars);
    let alone = path.with_file(FileKind::SealedBars).to_path_buf(&other.0);
    fs::create_dir_all(alone.parent().unwrap()).unwrap();
    fs::write(&alone, bytes).unwrap();
    assert!(!at.exists());
    let sealed = BarFile::open_existing(&other.0, path, SYMBOL).unwrap();
    assert_eq!(sealed.form(), Form::Sealed);
    assert_eq!(sealed.read_range(0, 700).unwrap(), bars);
    assert!(
        !path
            .with_file(FileKind::Lock)
            .to_path_buf(&other.0)
            .exists(),
        "a sealed month takes no lock and creates none"
    );
}

#[test]
fn a_month_retired_between_two_opens_is_still_found() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // The first open finds no `.sbar`; before the second, the sealer's file
    // lands and the raw form is retired; the third open finds the sealed one.
    let root = Root::new("retired-between");
    let path = month_path(None);
    let bars = seconds(400);
    write_raw(&root.0, path, &bars);
    let staged = root.0.join("staged.sbar");
    fs::write(&staged, sealed_bytes(&root.0, path)).unwrap();
    let sbar = root.file(path, FileKind::SealedBars);
    let bin = root.file(path, FileKind::Bars);
    let retired = root.0.join("retired.bin");
    {
        let (sbar, bin, retired) = (sbar.clone(), bin.clone(), retired.clone());
        arm_between_opens(move || {
            fs::rename(&staged, &sbar).unwrap();
            fs::rename(&bin, &retired).unwrap();
        });
    }
    let file = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(file.form(), Form::Sealed, "the third open found it");
    assert_eq!(file.read_range(0, 400).unwrap(), bars);
    assert!(!bin.exists() && retired.exists() && sbar.exists());

    // Neither form: the raw door's own `Missing`, naming the `.bin`.
    fs::rename(&sbar, root.0.join("gone.sbar")).unwrap();
    match BarFile::open_existing(&root.0, path, SYMBOL) {
        Err(StoreError::Missing { path: missing, .. }) => assert_eq!(missing, bin),
        other => panic!("expected Missing naming the .bin, got {other:?}"),
    }
}

#[test]
fn a_reader_refused_by_a_retirement_finds_the_sealed_month() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("retiring");
    let path = month_path(None);
    let bars = seconds(300);
    write_raw(&root.0, path, &bars);
    let lock_path = root.file(path, FileKind::Lock);
    // THE RETIREMENT'S EXCLUSIVE LOCK, on its own open file description.
    let held = fs::OpenOptions::new().read(true).open(&lock_path).unwrap();
    held.try_lock().unwrap();

    // No sealed form: the refusal is the raw door's, after one more probe.
    assert!(matches!(
        BarFile::open_existing(&root.0, path, SYMBOL),
        Err(StoreError::Locked { .. })
    ));

    // The sealed form lands while the first probe is past: the refused reader
    // probes once more and serves it.
    let staged = root.0.join("staged.sbar");
    fs::write(&staged, sealed_bytes_unlocked(&root.0, path, &bars)).unwrap();
    let sbar = root.file(path, FileKind::SealedBars);
    {
        let sbar = sbar.clone();
        arm_between_opens(move || fs::rename(&staged, &sbar).unwrap());
    }
    let file = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(file.form(), Form::Sealed);
    assert_eq!(file.read_range(0, 300).unwrap(), bars);
    drop(held);
}

/// [`sealed_bytes`] for a month whose lock another holder has: the header and
/// bars are taken from what was written, not through the door.
fn sealed_bytes_unlocked(root: &Path, path: StorePath<'_>, bars: &[Bar]) -> Vec<u8> {
    let header = {
        let other = Root::new("unlocked-header");
        write_raw(&other.0, path, bars);
        BarFile::open_existing(&other.0, path, SYMBOL)
            .unwrap()
            .header()
    };
    let prv = fs::read(path.with_file(FileKind::Provenance).to_path_buf(root)).ok();
    crate::sealed::build(path, &header, bars, prv.as_deref(), None)
        .unwrap()
        .to_bytes()
}

#[test]
fn a_damaged_sealed_month_is_refused_by_name_and_never_answered_from_the_bin() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("damaged");
    let path = month_path(None);
    let bars = seconds(1_000);
    write_raw(&root.0, path, &bars);
    let good = sealed_bytes(&root.0, path);
    let sbar = root.file(path, FileKind::SealedBars);

    // A FRAME BYTE: the head opens, the frame's own check refuses its rows.
    let mut bytes = good.clone();
    let last = bytes.len() - 3;
    bytes[last] ^= 0x10;
    fs::write(&sbar, &bytes).unwrap();
    let file = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(file.form(), Form::Sealed);
    assert!(
        file.read_record(0).is_ok(),
        "frame 0 is not the damaged one"
    );
    match file.read_record(999) {
        Err(StoreError::SealedRefused {
            path: at,
            reason,
            detail,
        }) => {
            assert_eq!(at, sbar);
            assert!(
                !reason.is_empty() && !detail.is_empty(),
                "{reason}: {detail}"
            );
            let shown = StoreError::SealedRefused {
                path: at,
                reason,
                detail,
            }
            .to_string();
            assert!(shown.contains("sealed month is refused"), "{shown}");
            assert!(shown.contains("not served"), "{shown}");
        }
        other => panic!("a damaged frame must be refused by name, got {other:?}"),
    }
    assert!(matches!(
        file.read_range(900, 100),
        Err(StoreError::SealedRefused { .. })
    ));
    assert!(matches!(
        file.read_rows_at(&[999]),
        Err(StoreError::SealedRefused { .. })
    ));
    // A refusal caches nothing: asked again, it is refused again.
    assert!(file.read_record(999).is_err());

    // A HEAD BYTE: the open itself refuses, and the `.bin` is not opened
    // instead.
    let mut bytes = good;
    bytes[30] ^= 0x01;
    fs::write(&sbar, &bytes).unwrap();
    match BarFile::open_existing(&root.0, path, SYMBOL) {
        Err(StoreError::SealedRefused { path: at, .. }) => assert_eq!(at, sbar),
        other => panic!("a damaged head must be refused, got {other:?}"),
    }

    // A TRUNCATED FILE: refused by its length.
    fs::write(&sbar, &bytes[..100]).unwrap();
    assert!(matches!(
        BarFile::open_existing(&root.0, path, SYMBOL),
        Err(StoreError::SealedRefused {
            reason: "too_short",
            ..
        })
    ));
}

#[test]
fn a_misfiled_sealed_month_is_refused() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // The sealed month of NIFTY spot copied beside another month, another
    // rung and another symbol id: the head binds itself to its path.
    let root = Root::new("misfiled");
    let path = month_path(None);
    write_raw(&root.0, path, &seconds(50));
    let bytes = sealed_bytes(&root.0, path);
    let june = path_at(None, Timeframe::SECOND_1, YearMonth::new(2031, 6).unwrap());
    let minute = path_at(None, Timeframe::MINUTE_1, YearMonth::new(2031, 5).unwrap());
    for other in [june, minute] {
        let at = other.with_file(FileKind::SealedBars).to_path_buf(&root.0);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(&at, &bytes).unwrap();
        assert!(
            matches!(
                BarFile::open_existing(&root.0, other, SYMBOL),
                Err(StoreError::SealedRefused {
                    reason: "misfiled",
                    ..
                })
            ),
            "{other}"
        );
    }
    let at = root.file(path, FileKind::SealedBars);
    fs::write(&at, &bytes).unwrap();
    assert!(matches!(
        BarFile::open_existing(&root.0, path, SYMBOL + 1),
        Err(StoreError::SealedRefused {
            reason: "misfiled",
            ..
        })
    ));
    assert!(BarFile::open_existing(&root.0, path, SYMBOL).is_ok());
}

#[test]
fn a_sealed_month_needs_no_time_index_and_none_is_written() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("no-tix");
    let path = month_path(None);
    write_raw(&root.0, path, &seconds(2_000));
    seal(&root.0, path);
    let tix = root.file(path, FileKind::TimeIndex);
    let before = fs::read(&tix).ok();
    let done = BarFile::index_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(done.outcome, Reindexed::Sealed);
    assert_eq!(done.n_valid, 2_000);
    assert_eq!(fs::read(&tix).ok(), before, "nothing was written");
}

#[test]
fn the_audited_door_audits_both_forms_and_compares_them() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("audited");
    let path = month_path(None);
    let bars = seconds(900);
    write_raw(&root.0, path, &bars);
    let good = sealed_bytes(&root.0, path);
    seal(&root.0, path);
    let audited = BarFile::open_existing_audited(&root.0, path, SYMBOL, u64::MAX).unwrap();
    assert_eq!(audited.evidence().header().n_valid, 900);

    // A sealed form of OTHER bars beside the raw one: both audit alone, and
    // they disagree.
    let other = Root::new("audited-other");
    let mut changed = bars.clone();
    changed[17].close += 1;
    write_raw(&other.0, path, &changed);
    let foreign = sealed_bytes(&other.0, path);
    let sbar = root.file(path, FileKind::SealedBars);
    fs::write(&sbar, &foreign).unwrap();
    let refused = BarFile::open_existing_audited(&root.0, path, SYMBOL, u64::MAX)
        .err()
        .unwrap();
    assert!(refused.contains("disagree"), "{refused}");

    // A damaged sealed form refuses the audit by name.
    let mut damaged = sealed_bytes(&other.0, path);
    let at = damaged.len() - 5;
    damaged[at] ^= 0x40;
    fs::write(&sbar, &damaged).unwrap();
    let refused = BarFile::open_existing_audited(&root.0, path, SYMBOL, u64::MAX)
        .err()
        .unwrap();
    assert!(refused.contains("sealed month is refused"), "{refused}");

    // The sealed form alone: audited, and no receipt is invented for a raw
    // form that is not there.
    let alone = Root::new("audited-alone");
    let at = path.with_file(FileKind::SealedBars).to_path_buf(&alone.0);
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    fs::write(&at, &good).unwrap();
    let refused = BarFile::open_existing_audited(&alone.0, path, SYMBOL, u64::MAX)
        .err()
        .unwrap();
    assert!(
        refused.contains("passed its full audit (900 rows"),
        "{refused}"
    );
    assert!(refused.contains("has none"), "{refused}");
}

#[test]
fn a_sealed_handle_is_shared_across_threads_and_answers_alike() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("threads");
    let path = month_path(None);
    let bars = seconds(5_000);
    write_raw(&root.0, path, &bars);
    seal(&root.0, path);
    let file = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    std::thread::scope(|scope| {
        for t in 0..4u64 {
            let (file, bars) = (&file, &bars);
            scope.spawn(move || {
                let mut i = t * 997 % 5_000;
                for _ in 0..2_000 {
                    assert_eq!(
                        file.read_record(i).unwrap(),
                        bars[usize::try_from(i).unwrap()]
                    );
                    i = (i * 31 + 7 + t) % 5_000;
                }
            });
        }
    });
}

#[test]
fn the_catalog_holds_a_month_in_either_form_once() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // Raw only (May), both forms (June), sealed alone (July).
    let root = Root::new("catalog");
    let elsewhere = Root::new("catalog-elsewhere");
    let (may, june, july) = (
        month_path(None),
        path_at(None, Timeframe::SECOND_1, YearMonth::new(2031, 6).unwrap()),
        path_at(None, Timeframe::SECOND_1, YearMonth::new(2031, 7).unwrap()),
    );
    write_raw(&root.0, may, &seconds(3));
    write_raw(&root.0, june, &shifted(&seconds(3), 31));
    seal(&root.0, june);
    write_raw(&elsewhere.0, july, &shifted(&seconds(3), 61));
    let alone = july.with_file(FileKind::SealedBars).to_path_buf(&root.0);
    fs::create_dir_all(alone.parent().unwrap()).unwrap();
    fs::write(&alone, sealed_bytes(&elsewhere.0, july)).unwrap();

    let found = crate::catalog::walk(&root.0).unwrap();
    let months: Vec<(u16, u8)> = found
        .held
        .iter()
        .map(|held| (held.month.year(), held.month.month()))
        .collect();
    assert_eq!(months, [(2031, 5), (2031, 6), (2031, 7)], "one row a month");
    assert_eq!(found.census.spot, 4, "four month files: two forms of June");
    assert_eq!(found.census.sealed, 2, "June's and July's sealed forms");
    assert!(found.census.reconciles(), "{:?}", found.census);
    assert_eq!(found.census.unoffered_report(), "");
}

/// `bars` moved `days` days later, into the next months of 2031.
fn shifted(bars: &[Bar], days: i64) -> Vec<Bar> {
    bars.iter()
        .map(|bar| Bar {
            ts_micros: bar.ts_micros + days * 86_400 * MICROS,
            ..*bar
        })
        .collect()
}

#[test]
fn a_sealed_month_carries_its_provenance_to_the_reader() {
    // EVERY TEST HERE APPENDS OR IS REFUSED, so it emits: the sink is held
    // (`crate::emits::hold_the_sink`).
    let _sink_is_mine = crate::emits::hold_the_sink();
    // The packed `.prv` image comes back through `provenance::read` exactly as
    // the raw month's sidecar reads, and a sealed month sealed without one is
    // `Absent`, as its raw form was.
    let root = Root::new("provenance");
    let path = month_path(None);
    write_raw(&root.0, path, &seconds(10));
    seal(&root.0, path);
    assert!(matches!(
        crate::provenance::read(&root.0, path),
        Ok(crate::provenance::Found::Absent)
    ));
    let prv = root.file(path, FileKind::Provenance);
    assert!(!prv.exists());

    // A month WITH a sidecar image: an unborn one (a header cut short) is
    // carried byte for byte and read back as unborn, by its length.
    let other = Root::new("provenance-image");
    write_raw(&other.0, path, &seconds(10));
    fs::write(other.file(path, FileKind::Provenance), [0u8; 40]).unwrap();
    let raw_answer = format!("{:?}", crate::provenance::read(&other.0, path));
    seal(&other.0, path);
    fs::rename(
        other.file(path, FileKind::Provenance),
        other.0.join("aside.prv"),
    )
    .unwrap();
    let sealed_answer = format!("{:?}", crate::provenance::read(&other.0, path));
    assert_eq!(
        sealed_answer, raw_answer,
        "the sealed section reads as the sidecar did"
    );
    assert!(sealed_answer.contains("Unborn"), "{sealed_answer}");
}
