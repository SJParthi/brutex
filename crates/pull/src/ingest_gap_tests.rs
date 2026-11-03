//! BASE-COV-PULL PS3: the store-writing doors' refusals no test reached. A
//! census that moves under its lock, a lock whose handle cannot be measured,
//! the repair and back-fill doors' judgements and journal lines, the derived
//! rungs a repair refolds, and the reads a refused month's audit makes, each
//! driven through the production function with real files and real host
//! refusals. Every symbol (`ZQIDX`) and every price is invented; the dates are
//! March and April 2025 trading days, which `crate::calendar` records.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::os::unix::fs::FileTypeExt as _;

use super::*;
use crate::fetch::{RawRow, Refused, RowCause};
use crate::gdfl_fixtures::scratch;
use crate::response_keep::RefusedRow;
use crate::session::Day;
use store::format::Overlay;
use store::path::YearMonth;
use store::repair::Policy;

/// The invented index every test files under.
const IDX: &str = "ZQIDX";

/// 09:15 IST, the open, in minutes past IST midnight.
const OPEN: i64 = 555;

/// A repair that compares no open interest and asks no basis witness.
const NO_WITNESS: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};

/// A reason every repair of these tests carries.
const REASON: &str = "PS3 gap test: invented bars";

fn civil(year: u16, month: u8, day: u8) -> Day {
    Day::new(year, month, day).unwrap()
}

/// A day of March 2025; the 3rd to the 7th, the 10th and the 11th traded.
fn mar(day: u8) -> Day {
    civil(2025, 3, day)
}

fn march() -> YearMonth {
    YearMonth::new(2025, 3).unwrap()
}

fn april() -> YearMonth {
    YearMonth::new(2025, 4).unwrap()
}

/// `minute` minutes past IST midnight on `on`, in the store's microseconds.
fn stamp(on: Day, minute: i64) -> i64 {
    (i64::from(on.days_from_epoch()) * 86_400 - crate::session::IST_OFFSET_SECS + minute * 60)
        * 1_000_000
}

/// An invented bar at `ts_micros`, priced about `price` paisa.
fn priced(ts_micros: i64, price: i64) -> Bar {
    Bar {
        ts_micros,
        open: price,
        high: price + 50,
        low: price - 50,
        close: price + 5,
        volume: 7,
        open_interest: i64::MIN,
    }
}

/// The open of `on`, as an invented bar.
fn opening(on: Day) -> Bar {
    priced(stamp(on, OPEN), 123_450)
}

fn parts_of(vendor: Vendor, timeframe: Timeframe, month: YearMonth) -> PathParts<'static> {
    PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol: IDX,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

fn key_of(timeframe: Timeframe, month: YearMonth) -> EntryKey {
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: brutex_core::symbol::Symbol::new(IDX).unwrap(),
        timeframe,
        month,
    }
}

fn path_of(parts: PathParts<'_>) -> StorePath<'_> {
    StorePath::new(parts).unwrap()
}

fn idx_id() -> u32 {
    identify(IDX, "NSE", "INDEX").unwrap().symbol_id
}

/// Files `bars` under `parts`, as the append door files them.
fn file_month(root: &Path, parts: PathParts<'_>, bars: &[Bar]) {
    let mut file = BarFile::open_or_create(root, path_of(parts), idx_id()).unwrap();
    let _appended = file.append(bars).unwrap();
}

/// A month made and holding nothing: its genesis header alone.
fn made_empty(root: &Path, parts: PathParts<'_>) {
    drop(BarFile::open_or_create(root, path_of(parts), idx_id()).unwrap());
}

/// Three bytes where the month's bars belong: a month that will not open.
fn planted_cut(root: &Path, parts: PathParts<'_>) {
    let file = path_of(parts).to_path_buf(root);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, b"cut").unwrap();
}

/// Every bar of the month at `parts`, as every reader opens it.
fn held_bars(root: &Path, parts: PathParts<'_>) -> Vec<Bar> {
    let file = BarFile::open_existing(root, path_of(parts), idx_id()).unwrap();
    (0..file.header().n_valid)
        .map(|index| file.read_record(index).unwrap())
        .collect()
}

/// The generation of `parts` every reader opens: `0` the original.
fn generation(root: &Path, parts: PathParts<'_>) -> u32 {
    store::generation::served(root, path_of(parts))
        .unwrap()
        .ordinal()
}

/// A directory where `vendor`'s repair journal belongs: it takes no line.
fn block_journal(root: &Path, vendor: Vendor) {
    let journal = root
        .join("audit")
        .join(format!("{}.repairs", vendor.as_str()));
    fs::create_dir_all(journal).unwrap();
}

/// The lines of `vendor`'s repair journal whose first word is `verb`.
fn journaled(root: &Path, vendor: Vendor, verb: &str) -> usize {
    let journal = root
        .join("audit")
        .join(format!("{}.repairs", vendor.as_str()));
    fs::read_to_string(journal)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.split(' ').next() == Some(verb))
        .count()
}

fn request(from: Day, to: Day) -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: crate::vendor::Listing::Index,
        window: crate::session::Window::new(from, to).unwrap(),
        granularity: crate::vendor::Granularity::Minute1,
    }
}

fn plan_for(request: &BarRequest, vendor: Vendor) -> Plan<'_> {
    Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        columns: Columns::Gdfl,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 0,
    }
}

/// One invented vendor row at the open of `on`.
fn member_at(on: Day) -> Member {
    Member {
        path: PathBuf::from("PS3 GAP ORIGIN"),
        instrument: IDX.to_owned(),
        rows: vec![RawRow {
            timestamp: stamp(on, OPEN) / 1_000_000,
            open: 123_450,
            high: 123_500,
            low: 123_400,
            close: 123_455,
            volume: 3,
            open_interest: None,
        }],
    }
}

fn derive_into(vendor: Vendor, month: YearMonth) -> DeriveInto<'static> {
    DeriveInto {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        contract: None,
        vendor,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: brutex_core::symbol::Symbol::new(IDX).unwrap(),
        month,
    }
}

/// The repair door's derive of March's minutes, as a repair run calls it.
fn derive_again(
    root: &Path,
    source: Timeframe,
    into: DeriveInto<'_>,
    entries: &mut Vec<Held>,
) -> Vec<Failure> {
    derive_repair(
        IDX,
        root,
        idx_id(),
        source,
        (into, NO_WITNESS, REASON),
        entries,
    )
}

/// An append refusal of `kind`, in invented words.
fn refusal(kind: FailKind) -> DoorRefusal {
    DoorRefusal {
        kind,
        why: "an invented append refusal".to_owned(),
    }
}

// ═══ The census a run writes to ═════════════════════════════════════════════

/// **A CENSUS THAT CANNOT SAY WHERE IT LIVES STOPS THE RUN UNWRITTEN.** A file
/// where the vendor's version-4 directory belongs, and a migrated vendor whose
/// plan names a segment the store does not define: the run is refused against
/// the census, with the rows it read counted and not one bar written. EXTREME
/// (each); together the two shapes of `target_for_plan`'s refusal.
#[test]
fn a_census_that_cannot_be_located_stops_the_run_before_a_bar_is_written() {
    let req = request(mar(3), mar(3));
    let root = scratch("gap-census-file");
    fs::create_dir_all(root.join("manifest").join("v4")).unwrap();
    fs::write(root.join("manifest/v4/zerodha"), b"not a directory").unwrap();
    let done = from_members(&[member_at(mar(3))], &root, plan_for(&req, Vendor::Zerodha));
    assert_eq!((done.members, done.rows_read, done.bars_stored), (1, 1, 0));
    assert_eq!(done.failures.len(), 1);
    let census = crate::manifest::manifest_path(&root, Vendor::Zerodha);
    assert_eq!(done.failures[0].instrument, census.display().to_string());
    assert!(
        done.failures[0].why.contains("is not a directory"),
        "{done:?}"
    );
    assert!(!root.join("bars").exists(), "nothing was written");
    fs::remove_dir_all(&root).unwrap();

    let root = scratch("gap-census-segment");
    fs::create_dir_all(root.join("manifest/v4/zerodha")).unwrap();
    let odd = Plan {
        segment: "ZQSEG",
        ..plan_for(&req, Vendor::Zerodha)
    };
    let done = from_members(&[member_at(mar(3))], &root, odd);
    assert_eq!(done.failures.len(), 1);
    assert!(
        done.failures[0].why.starts_with("segment \"ZQSEG\": "),
        "{done:?}"
    );
    assert!(!root.join("bars").exists(), "nothing was written");
    fs::remove_dir_all(&root).unwrap();
}

/// **A CENSUS THAT MOVES UNDER ITS LOCK IS REFUSED BEFORE IT IS READ.** OUT OF
/// THE BOX: the single file's lock is a symbolic link into the version-4
/// tree, so TAKING the lock creates `manifest/v4/zerodha` — exactly the race
/// with `migrate` the second look exists for. The run located a single file,
/// locked, looked again, and stopped with nothing written.
#[test]
fn a_census_that_moves_under_its_lock_is_refused_before_it_is_read() {
    let req = request(mar(3), mar(3));
    let root = scratch("gap-census-moved");
    fs::create_dir_all(root.join("manifest").join("v4")).unwrap();
    std::os::unix::fs::symlink("v4/zerodha", root.join("manifest/zerodha.man.lock")).unwrap();
    let done = from_members(&[member_at(mar(3))], &root, plan_for(&req, Vendor::Zerodha));
    assert_eq!((done.rows_read, done.bars_stored, done.counted), (1, 0, 0));
    assert_eq!(done.failures.len(), 1);
    let census = crate::manifest::manifest_path(&root, Vendor::Zerodha);
    assert_eq!(done.failures[0].instrument, census.display().to_string());
    assert!(
        done.failures[0]
            .why
            .contains("is not a directory, and it is where zerodha's version-4 census lives"),
        "{done:?}"
    );
    let made = fs::symlink_metadata(root.join("manifest/v4/zerodha")).unwrap();
    assert!(made.is_file(), "the lock's own open made it");
    assert!(!root.join("bars").exists(), "nothing was written");
    assert!(!census.exists(), "and no census either");
    fs::remove_dir_all(&root).unwrap();
}

/// No census rows is no census cycle: nothing is locked, read or made.
#[test]
fn no_census_rows_records_nothing_and_makes_nothing() {
    let root = scratch("gap-held-empty");
    assert_eq!(record_held(&root, Vendor::Zerodha, &[]), None);
    assert!(
        !root.join("manifest").exists(),
        "no lock, no census, no directory"
    );
    fs::remove_dir_all(&root).unwrap();
}

// ═══ The census lock ════════════════════════════════════════════════════════

/// **A LOCK THAT OPENED AND CANNOT BE MEASURED IS A REFUSAL.** The handle's own
/// answer decides: a regular file is locked, anything else is named, and a
/// handle the host will not measure refuses rather than run unlocked.
#[test]
fn a_lock_handle_is_judged_by_its_own_answer() {
    let lock = Path::new("/store/manifest/zerodha.man.lock");
    let why = CensusLock::opened(
        lock,
        Err(std::io::Error::other("an invented fstat refusal")),
    )
    .unwrap_err();
    assert_eq!(
        why,
        "the census lock at /store/manifest/zerodha.man.lock opened but cannot be measured: \
         an invented fstat refusal. Refused rather than run without it."
    );
    let root = scratch("gap-lock-opened");
    let regular = root.join("zerodha.man.lock");
    fs::write(&regular, b"").unwrap();
    assert_eq!(CensusLock::opened(&regular, fs::metadata(&regular)), Ok(()));
    let device = CensusLock::opened(lock, fs::metadata("/dev/null")).unwrap_err();
    assert!(
        device.contains("is not a regular file (a character device)"),
        "{device}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// **A LOCK WHOSE DIRECTORY CANNOT BE MEASURED REFUSES WITH BOTH REASONS.** A
/// symbolic link that loops sits where the manifest's parent belongs: the open
/// fails, the name is not there to be judged, and the directory's own `stat`
/// fails for a reason that is not "there is nothing here". EXTREME.
#[test]
fn a_lock_whose_directory_cannot_be_measured_refuses_with_both_reasons() {
    let root = scratch("gap-lock-loop");
    std::os::unix::fs::symlink("loop", root.join("loop")).unwrap();
    let census = root.join("loop").join("manifest").join("zerodha.man");
    let why = CensusLock::take(&census).err().unwrap();
    assert!(
        why.contains("and the directory that holds it could not be measured: "),
        "{why}"
    );
    assert!(
        why.ends_with("Refused rather than run without the lock."),
        "{why}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// **A CENSUS PATH WITH NO DIRECTORY.** EXTREME, TWICE: the empty path has no
/// directory and no name, so nothing can hold the census and the guard holds
/// nothing (two such guards stand side by side); the root names a directory,
/// which is refused, never locked. A COMBINATION of no parent with each.
#[test]
fn a_census_path_with_no_directory_defers_or_refuses_by_what_is_there() {
    let first = CensusLock::take(Path::new("")).unwrap();
    let second = CensusLock::take(Path::new("")).unwrap();
    drop((first, second));
    let why = CensusLock::take(Path::new("/")).err().unwrap();
    assert!(why.contains("exists but cannot be opened"), "{why}");
}

/// A block device at a name is named as one. The premise is the host's: the
/// first block device under `/dev`.
#[test]
fn a_block_device_is_named_as_one() {
    let device = fs::read_dir("/dev")
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.file_type().is_ok_and(|kind| kind.is_block_device()))
        .expect("the premise: this host has a block device under /dev");
    let found = fs::symlink_metadata(device.path()).unwrap();
    assert_eq!(entry_kind(found.file_type()), "a block device");
}

/// **A REGULAR FILE THAT READS PAST ITS OWN LENGTH IS REFUSED AT THE CAP.**
/// EXTREME: procfs measures `/proc/self/status` as empty and serves bytes, the
/// shape of a file that grew between the `fstat` and the read. A cap it fits
/// under reads it whole.
#[cfg(target_os = "linux")]
#[test]
fn a_file_that_reads_past_its_own_length_is_refused_at_the_cap() {
    let status = Path::new("/proc/self/status");
    let measured = fs::metadata(status).unwrap();
    assert!(measured.is_file(), "the premise: a regular file");
    assert_eq!(measured.len(), 0, "the premise: measured as empty");
    assert_eq!(
        read_regular_capped(status, 3).unwrap(),
        Err(Unbounded::PastCap { cap: 3, size: 4 })
    );
    let whole = read_regular_capped(status, 1 << 20).unwrap().unwrap();
    assert!(whole.starts_with(b"Name:"), "{whole:?}");
}

// ═══ Rows refused by name ═══════════════════════════════════════════════════

/// **REFUSED ROWS WHOSE CAUSES DO NOT PAIR ARE COUNTED WHOLE.** By the longer
/// of the two lists, either way round; at the ceiling the count saturates
/// rather than wraps (EXTREME); a paired record still lands row by row.
#[test]
fn unpaired_refusals_are_counted_by_the_longer_list_and_saturate() {
    let more_stamps = Refused {
        local_seconds: vec![11, 12, 13],
        causes: vec![RowCause::Price],
        ..Refused::default()
    };
    assert_eq!(named_refusals(&more_stamps), (Vec::new(), 3));
    let more_causes = Refused {
        local_seconds: Vec::new(),
        causes: vec![RowCause::OffGrid, RowCause::Conflicting],
        ..Refused::default()
    };
    assert_eq!(named_refusals(&more_causes), (Vec::new(), 2));
    let (mut into, mut unpaired) = (Vec::new(), u32::MAX - 1);
    pair_refused(&more_stamps, &mut into, &mut unpaired);
    assert_eq!((into.len(), unpaired), (0, u32::MAX));
    let paired = Refused {
        local_seconds: vec![21, 22],
        causes: vec![RowCause::Price, RowCause::OffGrid],
        ..Refused::default()
    };
    pair_refused(&paired, &mut into, &mut unpaired);
    assert_eq!(unpaired, u32::MAX);
    assert_eq!(
        into,
        [
            RefusedRow {
                local_second: 21,
                cause: RowCause::Price
            },
            RefusedRow {
                local_second: 22,
                cause: RowCause::OffGrid
            }
        ]
    );
}

// ═══ The decoded-bar doors ══════════════════════════════════════════════════

/// **A DECODED BAR THE CALENDAR CANNOT PLACE IS A COUNTED DROP.** EXTREME: a
/// stamp three days before the epoch. And a batch of nothing files nothing.
#[test]
fn a_bar_the_calendar_cannot_place_is_dropped_by_count_and_nothing_is_filed() {
    let req = request(mar(3), mar(3));
    let root = scratch("gap-before-epoch");
    let lost = priced(-3 * 86_400 * 1_000_000, 123_450);
    let done = from_rows(
        &[lost],
        &[],
        IDX,
        "PS3 GAP",
        &root,
        plan_for(&req, Vendor::Zerodha),
    );
    let before = done.census.of(crate::session::DropReason::BeforeWindow);
    assert_eq!(before, 1);
    assert_eq!((done.rows_read, done.bars_stored, done.counted), (1, 0, 0));
    assert!(done.failures.is_empty(), "{done:?}");
    assert!(done.balances());
    let (nothing, filed) = repair_rows(
        &[],
        IDX,
        "PS3 GAP",
        &root,
        plan_for(&req, Vendor::Gdfl),
        REASON,
    );
    assert_eq!(filed, Filed::Appended);
    assert_eq!((nothing.rows_read, nothing.counted), (0, 0));
    assert!(nothing.failures.is_empty(), "{nothing:?}");
    assert!(!root.join("bars").exists(), "nothing was filed");
    fs::remove_dir_all(&root).unwrap();
}

/// **AN OVERLAY THAT WILL NOT FILE AFTER ITS BARS IS NAMED, WITH WHERE THE ROWS
/// CAME FROM.** A directory sits where the overlay file belongs: the bars land
/// and are counted, and the one failure names the month and the origin.
#[test]
fn an_overlay_that_will_not_file_is_named_with_its_rows_origin() {
    let req = request(mar(3), mar(3));
    let root = scratch("gap-overlay");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let overlay = path_of(PathParts {
        file: FileKind::Overlay,
        ..parts
    });
    fs::create_dir_all(overlay.to_path_buf(&root)).unwrap();
    let bars = [opening(mar(3)), priced(stamp(mar(3), OPEN + 1), 123_455)];
    let overlays = [Overlay {
        ts_micros: bars[0].ts_micros,
        spot: 123_450,
        iv_micros: 125_000,
    }];
    let done = from_rows(
        &bars,
        &overlays,
        IDX,
        "PS3 GAP ORIGIN",
        &root,
        plan_for(&req, Vendor::Zerodha),
    );
    assert_eq!(
        (done.bars_stored, done.bars_committed, done.counted),
        (2, 2, 1)
    );
    assert!(done.pending.is_some(), "the census row is handed back");
    assert_eq!(done.failures.len(), 1, "{done:?}");
    assert_eq!(done.failures[0].month, Some(march()));
    assert!(
        done.failures[0].why.ends_with(" (from PS3 GAP ORIGIN)"),
        "{done:?}"
    );
    assert_eq!(held_bars(&root, parts), bars);
    fs::remove_dir_all(&root).unwrap();
}

// ═══ The month a refused batch is judged against ════════════════════════════

/// **THE SERVED MONTH SAYS WHAT IT LACKS.** Nothing offered, a batch out of
/// order or repeating a stamp, a month not there and one holding nothing are
/// the append door's; a day between held days is lacked; a held bar inside
/// the span overlaps; a month that will not open is the store's refusal; and
/// a batch after a REVISION's last bar is lacked, since a revision is never
/// appended to. EXTREME (empty, unordered, duplicated, empty month,
/// unopenable); COMBINATION: a revision AND a batch past every held bar.
#[test]
fn the_served_month_says_what_it_lacks_of_a_batch() {
    let root = scratch("gap-lack");
    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march());
    let held = [
        opening(mar(3)),
        priced(stamp(mar(3), OPEN + 2), 123_460),
        opening(mar(10)),
    ];
    file_month(&root, parts, &held);
    let (path, id) = (path_of(parts), idx_id());
    let lack = |bars: &[Bar]| lack_of(&root, path, id, bars).unwrap();
    let (early, late) = (opening(mar(5)), priced(stamp(mar(5), OPEN + 1), 123_455));
    assert_eq!(lack(&[]), Lack::AppendDoors);
    assert_eq!(lack(&[late, early]), Lack::AppendDoors);
    assert_eq!(lack(&[early, early]), Lack::AppendDoors);
    assert_eq!(lack(&[early, late]), Lack::Lacked { served: 0 });
    let inside = [
        priced(stamp(mar(3), OPEN + 1), 123_455),
        priced(stamp(mar(3), OPEN + 3), 123_465),
    ];
    assert_eq!(
        lack(&inside),
        Lack::Overlaps {
            held_at: held[1].ts_micros
        }
    );
    let tail = [opening(mar(11))];
    assert_eq!(lack(&tail), Lack::AppendDoors);
    let april_parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, april());
    let april_bar = [opening(civil(2025, 4, 1))];
    let in_april = |bars: &[Bar]| lack_of(&root, path_of(april_parts), id, bars);
    assert_eq!(in_april(&april_bar).unwrap(), Lack::AppendDoors);
    made_empty(&root, april_parts);
    assert_eq!(in_april(&april_bar).unwrap(), Lack::AppendDoors);
    let may_parts = parts_of(
        Vendor::Gdfl,
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 5).unwrap(),
    );
    planted_cut(&root, may_parts);
    let unread = lack_of(&root, path_of(may_parts), id, &[opening(civil(2025, 5, 2))]).unwrap_err();
    assert_eq!(
        FailKind::of_store(&unread),
        FailKind::StoreDamaged,
        "{unread}"
    );
    let _repaired = store::repair::repair_month(&root, path, id, &[early], NO_WITNESS).unwrap();
    assert_eq!(generation(&root, parts), 1);
    assert_eq!(lack(&tail), Lack::Lacked { served: 1 });
    fs::remove_dir_all(&root).unwrap();
}

/// **A `NotHeld` REFUSAL THE MONTH CANNOT JUDGE KEEPS ITS KIND.** A path the
/// store cannot name is not judged at all, and a month that will not open
/// leaves the append's kind standing, the words saying why it was not judged.
#[test]
fn a_not_held_refusal_the_month_cannot_judge_keeps_its_kind() {
    let root = scratch("gap-judged");
    let offered = [opening(mar(5))];
    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march());
    let unnamed = PathParts {
        segment: "../OUT",
        ..parts
    };
    let appended = refusal(FailKind::NotHeld);
    assert_eq!(
        judged(appended.clone(), &offered, &root, idx_id(), unnamed),
        appended
    );
    planted_cut(&root, parts);
    let said = judged(appended, &offered, &root, idx_id(), parts);
    assert_eq!(said.kind, FailKind::NotHeld);
    assert!(
        said.why.starts_with(
            "an invented append refusal; the month as served could not be read to tell \
             which hole the day is ("
        ),
        "{said:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// **A REFUSED MONTH'S DAYS ARE AUDITED AGAINST WHAT IT STORES.** A day before
/// the epoch is not held and names no front day (EXTREME: no day number
/// exists for it); a day before the first held one is a front.
#[test]
fn a_refused_day_before_the_epoch_is_not_held_and_names_no_front() {
    let root = scratch("gap-front");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    file_month(&root, parts, &[opening(mar(4))]);
    let lost = priced(-3 * 86_400 * 1_000_000, 123_450);
    let front = opening(mar(3));
    let (mut seconds, mut fronts) = (Vec::new(), Vec::new());
    not_held_days(
        &[lost, front],
        &root,
        idx_id(),
        parts,
        &mut seconds,
        &mut fronts,
    );
    let second = |bar: Bar| local_second_of(bar.ts_micros);
    assert_eq!(seconds, [second(lost), second(front)]);
    assert_eq!(fronts, [u32::try_from(local_day(front.ts_micros)).unwrap()]);
    let mut repaired = NotHeld::default();
    let next = opening(mar(4));
    repaired.repaired(&[
        lost.ts_micros,
        front.ts_micros,
        front.ts_micros + 60_000_000,
        next.ts_micros,
    ]);
    let day = |bar: Bar| u32::try_from(local_day(bar.ts_micros)).unwrap();
    assert_eq!(repaired.repaired, [day(front), day(next)]);
    fs::remove_dir_all(&root).unwrap();
}

/// The first `minutes` minutes of each of `days`, invented: a month of more
/// than 73 holds several of the store's checksum blocks, and one of 1,024 or
/// more holds a time index (D-4933).
fn sessions(days: &[Day], minutes: i64) -> Vec<Bar> {
    days.iter()
        .flat_map(|&on| {
            (0..minutes).map(move |minute| priced(stamp(on, OPEN + minute), 123_450 + minute * 5))
        })
        .collect()
}

/// Overwrites the stamp of record `index` of the month at `parts`, so its
/// block no longer matches its checksum: a record that will not read.
fn damaged(root: &Path, parts: PathParts<'_>, index: u64) {
    use std::os::unix::fs::FileExt as _;
    let at = store::layout::Layout::CURRENT.offset_of(index).unwrap();
    let bin = fs::OpenOptions::new()
        .write(true)
        .open(path_of(parts).to_path_buf(root))
        .unwrap();
    bin.write_all_at(&[0xA5; 8], at).unwrap();
}

/// **A HELD DAY'S OFFERED BARS ARE WALKED AGAINST THE BARS IT STORES.** One past
/// the day's last stored bar runs the walk off the day's end. A small month,
/// which has no time index, whose damaged record the lookup bisects onto
/// cannot even place the day (EXTREME). A month with a time index places the
/// day without a bar read and then meets the damaged record: the rest of the
/// day is not held, never held by guess (COMBINATION: an index that answers
/// AND a record that does not).
#[test]
fn a_held_days_offered_bars_are_walked_against_what_it_stores() {
    let root = scratch("gap-offered");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let path = path_of(parts);
    let held = [opening(mar(3)), priced(stamp(mar(3), OPEN + 1), 123_455)];
    file_month(&root, parts, &held);
    let second = |bar: Bar| local_second_of(bar.ts_micros);
    let after = priced(stamp(mar(3), OPEN + 2), 123_460);
    let file = BarFile::open_existing(&root, path, idx_id()).unwrap();
    let mut seconds = Vec::new();
    offered_not_stored(
        &file,
        (2, stamp(mar(3), 0)),
        &[held[0], after],
        &mut seconds,
    );
    assert_eq!(seconds, [second(after)]);
    drop(file);
    fs::remove_dir_all(&root).unwrap();

    let small = sessions(&[mar(3)], 150);
    let root = scratch("gap-offered-bisected");
    file_month(&root, parts, &small);
    let file = BarFile::open_existing(&root, path, idx_id()).unwrap();
    assert_eq!(
        file.time_lookup(),
        store::file::TimeLookup::Bisection(store::time_index::Why::BelowThreshold)
    );
    damaged(&root, parts, 75);
    let mut seconds = Vec::new();
    offered_not_stored(&file, (150, stamp(mar(3), 0)), &small[..2], &mut seconds);
    assert_eq!(seconds, [second(small[0]), second(small[1])]);
    drop(file);
    fs::remove_dir_all(&root).unwrap();

    let indexed = sessions(&[mar(3), mar(4), mar(5)], 375);
    let root = scratch("gap-offered-indexed");
    file_month(&root, parts, &indexed);
    let file = BarFile::open_existing(&root, path, idx_id()).unwrap();
    assert_eq!(file.time_lookup(), store::file::TimeLookup::Indexed);
    damaged(&root, parts, 375);
    let mut seconds = Vec::new();
    offered_not_stored(
        &file,
        (1_125, stamp(mar(4), 0)),
        &indexed[375..377],
        &mut seconds,
    );
    assert_eq!(seconds, [second(indexed[375]), second(indexed[376])]);
    drop(file);
    fs::remove_dir_all(&root).unwrap();
}

// ═══ The repair authority ═══════════════════════════════════════════════════

/// **A REPAIR OF NOTHING NEW ANSWERS THE MONTH AS SERVED.** Every offered bar
/// is held, so nothing is published and nothing is journaled.
#[test]
fn a_repair_of_nothing_new_answers_the_month_as_served() {
    let root = scratch("gap-nothing-new");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let held = [opening(mar(3)), opening(mar(10))];
    file_month(&root, parts, &held);
    let into = (parts, key_of(Timeframe::MINUTE_1, march()));
    let appended = refusal(FailKind::NotHeld);
    let (row, added, added_at) = repair_after(
        &appended,
        &held[..1],
        (&root, idx_id()),
        into,
        (NO_WITNESS, REASON),
    )
    .unwrap();
    assert_eq!((row.entry.rows, added, added_at), (2, 0, Vec::new()));
    assert_eq!(generation(&root, parts), 0);
    assert_eq!(journaled(&root, Vendor::Zerodha, "repair"), 0);
    fs::remove_dir_all(&root).unwrap();
}

/// **A REPAIRED MONTH WHOSE JOURNAL WILL NOT TAKE ITS LINE IS SERVED, AND SAYS
/// SO**; a refused repair whose journal will not take its line says both.
/// The journal is a directory: the host refuses the line, never a guess.
#[test]
fn a_repair_whose_journal_will_not_take_its_line_says_so() {
    let root = scratch("gap-unjournaled");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let held = [opening(mar(3)), opening(mar(10))];
    file_month(&root, parts, &held);
    block_journal(&root, Vendor::Zerodha);
    let into = (parts, key_of(Timeframe::MINUTE_1, march()));
    let hole = [opening(mar(5))];
    let served = repair_or_append(&hole, &root, idx_id(), into, (NO_WITNESS, REASON)).unwrap_err();
    assert_eq!(
        served.kind,
        FailKind::Io {
            kind: ErrorKind::Other,
            publication_may_be_visible: true
        }
    );
    assert!(
        served.why.starts_with(
            "2025-03: repaired to generation 1 and served, but its journal line was not written: "
        ),
        "{served:?}"
    );
    assert_eq!(generation(&root, parts), 1);
    assert_eq!(held_bars(&root, parts), [held[0], hole[0], held[1]]);
    let witness = Policy {
        basis_witness: true,
        ..NO_WITNESS
    };
    let unwitnessed = [opening(mar(6))];
    let appended = refusal(FailKind::NotHeld);
    let refused = repair_after(
        &appended,
        &unwitnessed,
        (&root, idx_id()),
        into,
        (witness, REASON),
    )
    .unwrap_err();
    assert_eq!(refused.kind, FailKind::RepairRefused);
    assert!(
        refused
            .why
            .starts_with("an invented append refusal; the versioned repair refused it too: "),
        "{refused:?}"
    );
    assert!(
        refused
            .why
            .contains("; and the repair journal line was not written: "),
        "{refused:?}"
    );
    assert_eq!(generation(&root, parts), 1, "nothing more was published");
    fs::remove_dir_all(&root).unwrap();
}

/// **A LATE DAY OF A MONTH SERVED FROM A REVISION IS ITS NEXT SUCCESSOR.** The
/// repair door files the hole as revision 1, then a day after its last bar —
/// which the writer door never appends to a revision — as revision 2.
#[test]
fn a_late_day_of_a_repaired_month_is_repaired_as_its_successor() {
    let root = scratch("gap-successor");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let held = [opening(mar(3)), opening(mar(10))];
    file_month(&root, parts, &held);
    let into = (parts, key_of(Timeframe::MINUTE_1, march()));
    let repair = |bars: &[Bar]| repair_or_append(bars, &root, idx_id(), into, (NO_WITNESS, REASON));
    let hole = [opening(mar(5))];
    assert_eq!(repair(&hole).unwrap().1, 1);
    let late = [opening(mar(11))];
    let (row, added, added_at) = repair(&late).unwrap();
    assert_eq!(
        (row.entry.rows, added, added_at),
        (4, 1, vec![late[0].ts_micros])
    );
    assert_eq!(generation(&root, parts), 2);
    assert_eq!(
        held_bars(&root, parts),
        [held[0], hole[0], held[1], late[0]]
    );
    assert_eq!(journaled(&root, Vendor::Zerodha, "repair"), 2);
    fs::remove_dir_all(&root).unwrap();
}

/// **A SUCCESSOR IS REFUSED BEFORE ANYTHING IS PUBLISHED** when the month's
/// served generation cannot be read (a directory where its pointer belongs)
/// and when the journal will not take the `repairing` line, which keeps the
/// append's kind. EXTREME (each); COMBINATION with a caller's own reason.
#[test]
fn a_successor_is_refused_before_anything_is_published() {
    let root = scratch("gap-successor-refused");
    let parts = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let held = [opening(mar(3))];
    file_month(&root, parts, &held);
    let into = (parts, key_of(Timeframe::MINUTE_1, march()));
    let late = [opening(mar(4))];
    let appended = refusal(FailKind::Sealed);
    let pointer = store::generation::pointer_path(&root, path_of(parts));
    fs::create_dir_all(&pointer).unwrap();
    let unread = successor_of_served(
        &appended,
        &late,
        (&root, idx_id()),
        into,
        (false, Follows::Sealed, None),
    )
    .unwrap_err();
    assert_eq!(unread.kind, FailKind::StoreDamaged);
    assert!(
        unread
            .why
            .starts_with("an invented append refusal; the generation served could not be read: "),
        "{unread:?}"
    );
    fs::remove_dir(&pointer).unwrap();
    block_journal(&root, Vendor::Zerodha);
    let follows = (false, Follows::Revision, Some(REASON));
    let unjournaled =
        successor_of_served(&appended, &late, (&root, idx_id()), into, follows).unwrap_err();
    assert_eq!(unjournaled.kind, FailKind::Sealed);
    assert!(
        unjournaled.why.contains(
            "; not filed as the month's successor generation: the repair journal would not take \
             the `repairing` line ("
        ),
        "{unjournaled:?}"
    );
    assert_eq!(generation(&root, parts), 0);
    assert_eq!(held_bars(&root, parts), held);
    fs::remove_dir_all(&root).unwrap();
}

// ═══ The back-fill door ═════════════════════════════════════════════════════

/// **A BUSY MONTH IS JUDGED BY ITS SHAPE AND STAYS BUSY.** A reader holds the
/// month: the append is refused busy, the month still reads, and a batch with
/// a held bar inside its span is answered busy, never judged to differ. A
/// writer holds it: the month cannot even be read to judge it. EXTREME
/// (both); COMBINATION: a busy month AND an overlapping batch.
#[test]
fn a_busy_month_is_judged_by_its_shape_and_stays_busy() {
    let root = scratch("gap-busy");
    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march());
    let held = [
        opening(mar(3)),
        priced(stamp(mar(3), OPEN + 2), 123_460),
        opening(mar(10)),
    ];
    file_month(&root, parts, &held);
    let into = (parts, key_of(Timeframe::MINUTE_1, march()));
    let door =
        |bars: &[Bar]| back_fill_or_append(bars, &root, idx_id(), into, (NO_WITNESS, REASON));
    let reader = BarFile::open_existing(&root, path_of(parts), idx_id()).unwrap();
    let overlapping = [
        priced(stamp(mar(3), OPEN + 1), 123_455),
        priced(stamp(mar(3), OPEN + 3), 123_465),
    ];
    let busy = door(&overlapping).unwrap_err();
    assert_eq!(busy.kind, FailKind::MonthBusy);
    assert!(!busy.why.contains("not back-filled"), "{busy:?}");
    drop(reader);
    let writer = BarFile::open_or_create(&root, path_of(parts), idx_id()).unwrap();
    let unread = door(&[opening(mar(5))]).unwrap_err();
    assert_eq!(unread.kind, FailKind::MonthBusy);
    assert!(
        unread
            .why
            .contains("; not back-filled: the month as served could not be read to judge it ("),
        "{unread:?}"
    );
    drop(writer);
    assert_eq!(held_bars(&root, parts), held);
    assert_eq!(generation(&root, parts), 0);
    fs::remove_dir_all(&root).unwrap();
}

/// **A BACK-FILL THE JOURNAL WILL NOT ANNOUNCE PUBLISHES NOTHING**, through the
/// GDFL door itself: the append's kind, the month and the reason on the one
/// failure, and the month as it was.
#[test]
fn a_back_fill_the_journal_will_not_announce_publishes_nothing() {
    let root = scratch("gap-intent");
    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march());
    let held = [opening(mar(3)), opening(mar(10))];
    file_month(&root, parts, &held);
    block_journal(&root, Vendor::Gdfl);
    let req = request(mar(5), mar(5));
    let (done, filed) = repair_rows(
        &[opening(mar(5))],
        IDX,
        "PS3 GAP",
        &root,
        plan_for(&req, Vendor::Gdfl),
        REASON,
    );
    assert_eq!(filed, Filed::Appended);
    assert_eq!(done.door_kind, Some(FailKind::NotHeld));
    assert_eq!((done.counted, done.pending), (0, None));
    assert_eq!(done.failures.len(), 1, "{done:?}");
    assert_eq!(done.failures[0].month, Some(march()));
    assert!(
        done.failures[0].why.contains(
            "; not back-filled: the repair journal would not take the `repairing` line ("
        ),
        "{done:?}"
    );
    assert_eq!(generation(&root, parts), 0);
    assert_eq!(held_bars(&root, parts), held);
    fs::remove_dir_all(&root).unwrap();
}

/// Sets the bars of `parts` stamped `named` aside, as the quarantine's own
/// door does: ledgered, then a generation without them.
fn quarantined(root: &Path, parts: PathParts<'_>, named: &[i64]) {
    let why = store::quarantine::Why {
        cause: store::quarantine::Cause::ForeignLevel,
        who: "PS3 gap test",
        reason: REASON,
    };
    let _applied =
        store::quarantine::apply_month(root, path_of(parts), idx_id(), named, why).unwrap();
}

/// **BARS THE QUARANTINE SET ASIDE ARE NEVER ADDED BACK, BY EITHER BRANCH OF
/// THE BACK-FILL DOOR.** A day set aside is offered again: lacked between two
/// held days, or a late day after a revision's last bar (its successor). Each
/// is repaired to nothing new and filed as appended, with no new generation.
/// COMBINATION: a revision, a quarantine and a re-offer at once.
#[test]
fn bars_the_quarantine_set_aside_are_never_added_back() {
    let root = scratch("gap-quarantine");
    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march());
    let (third, fifth, tenth) = (opening(mar(3)), opening(mar(5)), opening(mar(10)));
    file_month(&root, parts, &[third, fifth, tenth]);
    quarantined(&root, parts, &[fifth.ts_micros]);
    assert_eq!(held_bars(&root, parts), [third, tenth]);
    let req = request(mar(3), mar(11));
    let (done, filed) = repair_rows(
        &[fifth],
        IDX,
        "PS3 GAP",
        &root,
        plan_for(&req, Vendor::Gdfl),
        REASON,
    );
    assert_eq!(filed, Filed::Appended);
    assert!(done.failures.is_empty(), "{done:?}");
    assert_eq!(
        (done.bars_stored, done.bars_committed, done.counted),
        (1, 0, 1)
    );
    assert_eq!(generation(&root, parts), 1);
    assert_eq!(held_bars(&root, parts), [third, tenth]);

    let parts = parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, april());
    let (first, second) = (opening(civil(2025, 4, 1)), opening(civil(2025, 4, 2)));
    file_month(&root, parts, &[first, second]);
    quarantined(&root, parts, &[second.ts_micros]);
    assert_eq!(held_bars(&root, parts), [first]);
    let req = request(civil(2025, 4, 1), civil(2025, 4, 2));
    let (done, filed) = repair_rows(
        &[second],
        IDX,
        "PS3 GAP",
        &root,
        plan_for(&req, Vendor::Gdfl),
        REASON,
    );
    assert_eq!(filed, Filed::Appended);
    assert!(done.failures.is_empty(), "{done:?}");
    assert_eq!(
        (done.bars_stored, done.bars_committed, done.counted),
        (1, 0, 1)
    );
    assert_eq!(generation(&root, parts), 1);
    assert_eq!(held_bars(&root, parts), [first]);
    fs::remove_dir_all(&root).unwrap();
}

/// A reason is one ledger field whatever it holds: nothing is a dash (EXTREME),
/// every byte outside the field's set an underscore, and past 160 characters
/// it is cut (EXTREME).
#[test]
fn a_reason_is_one_ledger_field_whatever_it_holds() {
    assert_eq!(ledger_field(""), "-");
    assert_eq!(ledger_field("a b=c/d:e.f-g_h"), "a_b_c/d:e.f-g_h");
    assert_eq!(ledger_field("\u{20b9} \u{20b9}"), "___");
    assert_eq!(ledger_field(&"x".repeat(200)), "x".repeat(160));
}

// ═══ The derived rungs ══════════════════════════════════════════════════════

/// A rung folds only into the coarser rungs it divides: two minutes and three
/// minutes skip the rungs they do not (EXTREME), a second folds into every
/// intraday rung, and the hour and the day into none.
#[test]
fn a_rung_folds_only_into_the_coarser_rungs_it_divides() {
    assert_eq!(derived_count(Timeframe::MINUTE_2), 3);
    assert_eq!(derived_count(Timeframe::MINUTE_3), 3);
    assert_eq!(derived_count(Timeframe::SECOND_1), 8);
    assert_eq!(derived_count(Timeframe::MINUTE_60), 0);
    assert_eq!(derived_count(Timeframe::DAY_1), 0);
}

/// **A SOURCE MONTH THAT WILL NOT READ DERIVES NO RUNG, AND SAYS SO ONCE**,
/// under the month it is about. EXTREME: no source month at all.
#[test]
fn a_source_month_that_will_not_read_derives_no_rung() {
    let root = scratch("gap-derive-unread");
    let mut entries = Vec::new();
    let failures = derive_all(
        &[opening(mar(3))],
        IDX,
        &root,
        idx_id(),
        Timeframe::MINUTE_1,
        derive_into(Vendor::Zerodha, march()),
        &mut entries,
    );
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].month, Some(march()));
    assert!(failures[0].why.contains("2025-03"), "{failures:?}");
    assert!(entries.is_empty());
    assert!(!root.join("bars").exists(), "no rung was written");
    fs::remove_dir_all(&root).unwrap();
}

/// **THE REPAIR DOOR'S DERIVE ANSWERS ONLY A MINUTE SOURCE IT CAN READ.** GDFL,
/// an option and a daily source derive nothing; a minute source that is not
/// there is one failure; a venue with no verified session is refused by name.
#[test]
fn the_repair_doors_derive_answers_only_a_minute_source_it_can_read() {
    let root = scratch("gap-derive-repair");
    let mut entries = Vec::new();
    let zerodha = derive_into(Vendor::Zerodha, march());
    let option = DeriveInto {
        contract: brutex_core::instrument::Contract::parse("2025-03-27-2500000-CE"),
        ..zerodha
    };
    for (source, into) in [
        (Timeframe::MINUTE_1, derive_into(Vendor::Gdfl, march())),
        (Timeframe::MINUTE_1, option),
        (Timeframe::DAY_1, zerodha),
    ] {
        assert_eq!(derive_again(&root, source, into, &mut entries), Vec::new());
    }
    let missing = derive_again(&root, Timeframe::MINUTE_1, zerodha, &mut entries);
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert_eq!(missing[0].month, Some(march()));
    let bse = DeriveInto {
        exchange: Exchange::Bse,
        ..zerodha
    };
    let unattested = derive_again(&root, Timeframe::MINUTE_1, bse, &mut entries);
    assert_eq!(
        unattested[0].why,
        "no verified minute-completeness venue for BSE/INDEX; derived rungs not repaired"
    );
    let source = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    let nowhere = PathParts {
        exchange: "ZQX",
        ..source
    };
    let refused = served_minutes(&root, idx_id(), nowhere).unwrap_err();
    assert!(
        refused.starts_with("no verified minute-completeness venue for ZQX/INDEX"),
        "{refused}"
    );
    let rung = PathParts {
        exchange: "BSE",
        timeframe: Timeframe::MINUTE_2,
        ..source
    };
    let context = (
        key_of(Timeframe::MINUTE_2, march()),
        None,
        crate::calendar::Runtime::default(),
    );
    let refused = derive(
        &[opening(mar(3))],
        Timeframe::MINUTE_1,
        Timeframe::MINUTE_2,
        &root,
        idx_id(),
        rung,
        context,
    );
    assert_eq!(
        refused.unwrap_err(),
        "no verified minute-completeness venue for BSE/INDEX"
    );
    assert!(entries.is_empty());
    assert!(!root.join("bars").exists(), "no rung was written");
    fs::remove_dir_all(&root).unwrap();
}

/// **A SOURCE THAT FOLDS TO NOTHING, OR WILL NOT FOLD, REPAIRS NO RUNG.** One
/// minute of an index leaves every bucket incomplete (EXTREME: the smallest
/// source); two minutes each of the largest volume a bar can carry overflow
/// every bucket's sum (EXTREME), and each rung is refused by name.
#[test]
fn a_source_that_folds_to_nothing_or_will_not_fold_repairs_no_rung() {
    let root = scratch("gap-derive-fold");
    let source = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    file_month(&root, source, &[opening(mar(3))]);
    let mut entries = Vec::new();
    let into = derive_into(Vendor::Zerodha, march());
    let incomplete = derive_again(&root, Timeframe::MINUTE_1, into, &mut entries);
    assert!(entries.is_empty(), "{entries:?}");
    assert_eq!(
        incomplete.len(),
        derived_count(Timeframe::MINUTE_1),
        "{incomplete:?}"
    );
    fs::remove_dir_all(&root).unwrap();

    let root = scratch("gap-derive-overflow");
    let heavy = |minute| Bar {
        volume: i64::MAX,
        ..priced(stamp(mar(3), minute), 123_450)
    };
    file_month(&root, source, &[heavy(OPEN), heavy(OPEN + 1)]);
    let overflowed = derive_again(&root, Timeframe::MINUTE_1, into, &mut entries);
    assert!(entries.is_empty(), "{entries:?}");
    let rungs: Vec<&str> = overflowed
        .iter()
        .map(|failure| failure.why.split(':').next().unwrap())
        .collect();
    assert_eq!(
        rungs,
        ["2min", "3min", "5min", "10min", "15min", "30min", "60min"]
    );
    fs::remove_dir_all(&root).unwrap();
}

/// **A HELD DERIVED BAR THE FOLD DOES NOT HAVE IS REFUSED, AND JOURNALED.** The
/// journal will not take the line, so the rung's refusal says both
/// (COMBINATION: a held bar with no candidate AND a refused journal); and a
/// held rung-month that will not open is refused in the store's words.
#[test]
fn a_held_derived_bar_the_fold_lacks_is_refused_and_journaled() {
    let root = scratch("gap-derive-held");
    let source = parts_of(Vendor::Zerodha, Timeframe::MINUTE_1, march());
    file_month(
        &root,
        source,
        &[opening(mar(3)), priced(stamp(mar(3), OPEN + 1), 123_455)],
    );
    let rung = PathParts {
        timeframe: Timeframe::MINUTE_2,
        ..source
    };
    file_month(&root, rung, &[priced(stamp(mar(3), OPEN + 2), 123_460)]);
    block_journal(&root, Vendor::Zerodha);
    let mut entries = Vec::new();
    let failures = derive_again(
        &root,
        Timeframe::MINUTE_1,
        derive_into(Vendor::Zerodha, march()),
        &mut entries,
    );
    let refused: Vec<&Failure> = failures
        .iter()
        .filter(|failure| failure.why.starts_with("2min: historical derived bar at "))
        .collect();
    assert_eq!(refused.len(), 1, "{failures:?}");
    assert!(
        refused[0]
            .why
            .contains("; and the repair journal line was not written: "),
        "{failures:?}"
    );
    let three = PathParts {
        timeframe: Timeframe::MINUTE_3,
        ..source
    };
    planted_cut(&root, three);
    let unread = held_against_repair((&root, idx_id()), three, (&[], false), || None).unwrap_err();
    let store_said = BarFile::open_existing(&root, path_of(three), idx_id())
        .err()
        .unwrap();
    assert_eq!(unread, store_said.to_string(), "the store's own words");
    assert_eq!(fs::read(path_of(three).to_path_buf(&root)).unwrap(), b"cut");
    fs::remove_dir_all(&root).unwrap();
}

/// **A COMMITTED CASH RECORD OF ANOTHER MONTH IS REFUSED, NEVER COUNTED.** OUT
/// OF THE BOX: no store door writes one, since every door admits only stamps
/// inside the month, so the month is written whole and one record's stamp is
/// then moved into February with its block's checksum sealed again exactly as
/// the store seals it (CRC-32C of the block's committed bytes, four bytes at
/// `4·b` of the `.crc`): a file every reader accepts, holding what no writer
/// would.
#[test]
fn a_committed_cash_record_of_another_month_is_refused() {
    use std::os::unix::fs::FileExt as _;
    let root = scratch("gap-cash-misdated");
    let parts = PathParts {
        segment: "CASH",
        symbol: "ZQCASH",
        ..parts_of(Vendor::Gdfl, Timeframe::MINUTE_1, march())
    };
    let id = identify("ZQCASH", "NSE", "CASH").unwrap().symbol_id;
    let bars = [opening(mar(3)), opening(mar(4)), opening(mar(5))];
    let mut month = BarFile::open_or_create(&root, path_of(parts), id).unwrap();
    let _appended = month.append(&bars).unwrap();
    drop(month);
    let layout = store::layout::Layout::CURRENT;
    let bin = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path_of(parts).to_path_buf(&root))
        .unwrap();
    let moved = stamp(civil(2025, 2, 28), OPEN);
    bin.write_all_at(&moved.to_le_bytes(), layout.offset_of(1).unwrap())
        .unwrap();
    let mut covered = vec![0_u8; 3 * 56];
    bin.read_exact_at(&mut covered, layout.offset_of(0).unwrap())
        .unwrap();
    let sums = path_of(PathParts {
        file: FileKind::Checksums,
        ..parts
    });
    let sidecar = fs::OpenOptions::new()
        .write(true)
        .open(sums.to_path_buf(&root))
        .unwrap();
    sidecar
        .write_all_at(&store::crc::crc32c(&covered).to_le_bytes(), 0)
        .unwrap();
    let why = committed_cash_days(&root, Vendor::Gdfl, "ZQCASH", &[march()]).unwrap_err();
    assert_eq!(
        why,
        "ZQCASH 2025-03: committed source timestamp belongs to 2025-02-28"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// **A HELD RUNG BEHIND SEVERAL CANDIDATES IS NAMED ONCE, WITH THE COUNT AND
/// BOTH ENDS**, and its tail record offered again; a held rung-month that
/// holds nothing is no history to walk (EXTREME).
#[test]
fn a_held_rung_behind_several_candidates_is_named_once() {
    let root = scratch("gap-reconcile");
    let rung = parts_of(Vendor::Zerodha, Timeframe::MINUTE_2, march());
    let candidate = |minute| priced(stamp(mar(3), minute), 123_450);
    let held = candidate(OPEN + 4);
    file_month(&root, rung, &[held]);
    let (first, second) = (candidate(OPEN), candidate(OPEN + 2));
    let mut bars = vec![first, second, held];
    let said = reconcile_derived(&mut bars, (&root, idx_id()), rung, (|| None, false)).unwrap();
    assert_eq!(said.len(), 1, "{said:?}");
    let named = format!(
        "historical derived gap at {}: 2 folded source candidates from {} to {} are absent",
        first.ts_micros, first.ts_micros, second.ts_micros
    );
    assert!(said[0].starts_with(&named), "{said:?}");
    assert_eq!(bars, [held], "the tail's own record is offered again");
    let empty = PathParts {
        timeframe: Timeframe::MINUTE_3,
        ..rung
    };
    made_empty(&root, empty);
    let mut offered = vec![first];
    let said = reconcile_derived(&mut offered, (&root, idx_id()), empty, (|| None, false)).unwrap();
    assert_eq!(said, Vec::<String>::new());
    assert_eq!(offered, [first]);
    fs::remove_dir_all(&root).unwrap();
}

/// The two minutes of an invented session and its pulled day, cut by one paisa
/// at the open and the close (DB-B19's shape), filed under `root`.
fn cut_day(root: &Path) -> [Bar; 2] {
    let minute = |offset, open, high, low, close| Bar {
        ts_micros: stamp(mar(3), OPEN + offset),
        open,
        high,
        low,
        close,
        volume: 1,
        open_interest: i64::MIN,
    };
    let cut = Bar {
        ts_micros: stamp(mar(3), OPEN),
        open: 1_000_004,
        high: 1_000_150,
        low: 999_900,
        close: 1_000_049,
        volume: 2,
        open_interest: i64::MIN,
    };
    file_month(
        root,
        parts_of(Vendor::Zerodha, Timeframe::DAY_1, march()),
        &[cut],
    );
    [
        minute(0, 1_000_005, 1_000_100, 1_000_000, 1_000_010),
        minute(1, 1_000_010, 1_000_020, 999_900, 1_000_050),
    ]
}

/// **A PULLED DAY CUT TO THE PAISA IS COUNTED.** Zerodha's day is checked
/// against its own minutes and both cut fields are counted, which is what
/// gives the check its own `vendor_truncated` line (asserted in a sink below).
#[test]
fn a_pulled_day_cut_to_the_paisa_is_counted() {
    let root = scratch("gap-daycut");
    let minutes = cut_day(&root);
    let into = derive_into(Vendor::Zerodha, march());
    let report = day_check_of(&minutes, &root, idx_id(), &into)
        .unwrap()
        .unwrap();
    assert_eq!((report.vendor_truncated, report.differed), (2, 1));
    assert!(report.consistent(), "{report:?}");
    check_day(&minutes, IDX, &root, idx_id(), &into);
    fs::remove_dir_all(&root).unwrap();
}

/// **EVERY LINE THESE REFUSALS WRITE REACHES A SINK WITH ITS FIELDS.** OUT OF
/// THE BOX: run in a child that installs a sink at `Trace`, so the lines are
/// read back from the file, not inferred. Unpaired refusals name both
/// lengths; a cut day names its rule, its count and its first cut fields.
#[test]
fn every_line_these_refusals_write_reaches_a_sink() {
    crate::support::where_permission_binds(
        "ingest::gap_tests::every_line_these_refusals_write_reaches_a_sink",
        every_line_these_refusals_write_reaches_a_sink_body,
    );
}

fn every_line_these_refusals_write_reaches_a_sink_body() {
    let home = scratch("gap-sink");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");
    let unpaired = Refused {
        local_seconds: vec![11, 12, 13],
        causes: vec![RowCause::Price],
        ..Refused::default()
    };
    assert_eq!(named_refusals(&unpaired).1, 3);
    let root = scratch("gap-sink-store");
    let minutes = cut_day(&root);
    check_day(
        &minutes,
        IDX,
        &root,
        idx_id(),
        &derive_into(Vendor::Zerodha, march()),
    );
    let read = |target: &str| {
        telemetry::tail(
            &home,
            sink.keep_files(),
            &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(target),
        )
        .records
    };
    let refused = read("pull.refused");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(
        refused[0].message,
        "refused rows whose causes do not pair with their stamps"
    );
    let count = |record: &telemetry::Record, key: &str| {
        record.field(key).and_then(telemetry::OwnedValue::as_u64)
    };
    assert_eq!(
        (count(&refused[0], "seconds"), count(&refused[0], "causes")),
        (Some(3), Some(1))
    );
    let cut: Vec<telemetry::Record> = read("pull.daycheck")
        .into_iter()
        .filter(|record| {
            record.field("rule").and_then(telemetry::OwnedValue::as_str) == Some("vendor_truncated")
        })
        .collect();
    assert_eq!(cut.len(), 1, "{cut:?}");
    assert_eq!(cut[0].level, telemetry::Level::Warn);
    let text = |key: &str| {
        cut[0]
            .field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .map(str::to_owned)
    };
    assert_eq!(
        (text("instrument"), text("month")),
        (Some(IDX.to_owned()), Some("2025-03".to_owned()))
    );
    assert_eq!(count(&cut[0], "fields"), Some(2));
    assert!(
        text("why").is_some_and(|why| why.contains("open, close one paisa under")),
        "{cut:?}"
    );
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&home).unwrap();
}
