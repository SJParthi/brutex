//! `cli month-recover` (R3-20, D-7093): the importer's re-read of the days a
//! recovery drops, a power cut during a census compaction, and the verb's
//! own refusals and resumes.
//!
//! Every store here is a temporary directory of invented bars and journal
//! lines: prices near 12,345 rupees that NIFTY never traded at in these
//! months. Nothing reads or writes a real store.
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

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::repair::{MonthRepair, Policy};

use super::{LEDGER, REREAD_REASON, command_with};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
const OPEN: i64 = 555 * MINUTE;
const INDEX: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};

/// A scratch store of invented bars, removed when it drops.
struct Store {
    root: PathBuf,
    vendor: Vendor,
    segment: &'static str,
    month: YearMonth,
}

impl Store {
    fn new(vendor: Vendor) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-month-recover-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self {
            root,
            vendor,
            segment: "INDEX",
            month: YearMonth::new(2020, 3).unwrap(),
        }
    }

    fn path(&self) -> StorePath<'static> {
        self.path_of(FileKind::Bars)
    }

    fn path_of(&self, file: FileKind) -> StorePath<'static> {
        StorePath::new(PathParts {
            vendor: self.vendor,
            exchange: "NSE",
            segment: self.segment,
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: self.month,
            file,
        })
        .unwrap()
    }

    fn symbol_id() -> u32 {
        let [a, b, c, d, ..] = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    fn file(&self, kind: FileKind) -> PathBuf {
        self.path_of(kind).to_path_buf(&self.root)
    }

    fn commit(&self, bars: &[Bar]) {
        let mut writer =
            BarFile::open_or_create(&self.root, self.path(), Self::symbol_id()).unwrap();
        writer.append(bars).unwrap();
    }

    fn day(&self, day: i64, n: i64) -> Vec<Bar> {
        let start = self.month.ist_bounds_micros().0 + (day - 1) * DAY + OPEN;
        (0..n)
            .map(|k| {
                let price = 1_234_500 + (k % 50) * 5;
                Bar {
                    ts_micros: start + k * MINUTE,
                    open: price,
                    high: price + 40,
                    low: price - 40,
                    close: price + 10,
                    volume: 7 + k % 3,
                    open_interest: OI_NULL,
                }
            })
            .collect()
    }

    /// Day 2 (100 bars) committed and durable; day 3 (80 bars) committed in
    /// the open batch, and of its writes only the header slot reached the
    /// disk, with, when `resealed`, the checksum entry it re-sealed over the
    /// day-two tail block: the cut then reaches into day two.
    fn tear(&self, resealed: bool) {
        self.commit(&self.day(2, 100));
        let bin = fs::read(self.file(FileKind::Bars)).unwrap();
        let crc = fs::read(self.file(FileKind::Checksums)).unwrap();
        self.commit(&self.day(3, 80));
        let after_bin = fs::read(self.file(FileKind::Bars)).unwrap();
        let after_crc = fs::read(self.file(FileKind::Checksums)).unwrap();
        let mut torn_bin = bin;
        torn_bin[..64].copy_from_slice(&after_bin[..64]);
        let mut torn_crc = crc;
        if resealed {
            torn_crc[4..8].copy_from_slice(&after_crc[4..8]);
        }
        fs::write(self.file(FileKind::Bars), torn_bin).unwrap();
        fs::write(self.file(FileKind::Checksums), torn_crc).unwrap();
    }

    fn journal(&self, text: &str) {
        let path = pull::gdfl_import::journal_path(&self.root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn run(&self, form: &str) -> (u8, String) {
        let mut out = String::new();
        let root = self.root.clone();
        let code = command_with(
            &[
                self.vendor.as_str(),
                &format!("NSE/{}/NIFTY", self.segment),
                "1min",
                "2020-03",
                form,
            ],
            &move || Ok(root.clone()),
            &mut out,
        );
        (code, out)
    }

    fn served_bars(&self) -> Vec<Bar> {
        let file = BarFile::open_existing(&self.root, self.path(), Self::symbol_id()).unwrap();
        (0..file.records())
            .map(|at| file.read_record(at).unwrap())
            .collect()
    }

    /// The importer's re-read of one day at a month served from a revision:
    /// its successor generation, the door `pull::ingest` files it through.
    fn reread(&self, bars: &[Bar]) {
        match store::repair::repair_month(&self.root, self.path(), Self::symbol_id(), bars, INDEX)
            .unwrap()
        {
            MonthRepair::NothingNew { .. } | MonthRepair::Repaired { .. } => {}
        }
    }

    fn tree(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(&directory).unwrap() {
                let at = entry.unwrap().path();
                if at.is_dir() {
                    pending.push(at);
                } else {
                    found.insert(
                        at.strip_prefix(&self.root).unwrap().to_path_buf(),
                        fs::read(&at).unwrap(),
                    );
                }
            }
        }
        found
    }

    fn ledger(&self) -> String {
        fs::read_to_string(
            self.root
                .join("audit")
                .join(format!("{}.{LEDGER}", self.vendor.as_str())),
        )
        .unwrap_or_default()
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const JOURNAL: &str = "done indices 2020-03-02 * definition=4 files=1 seconds=100 failures=0\n\
                       begin indices 2020-03-03 *\n";

/// RA-280: THE DAYS DROPPED BY RECOVERY ARE READ AGAIN.
///
/// The cut reaches into day two, which the import journal shows done, so a
/// plain Resume would never read it again and its dropped bars would be a
/// silent hole. The door gives it one `incomplete` line in the journal's own
/// grammar with a reason `admit` reads on a plain Resume; day three, open
/// in the journal, is read again by any Resume already. After the re-read of
/// both days the month is the clean month. A second `--yes` writes nothing.
#[test]
fn the_days_dropped_by_recovery_are_read_again() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal(JOURNAL);
    let done = pull::gdfl_import::done_whole_days(&store.root).unwrap();
    assert!(done.contains("indices 2020-03-02"));

    let tree = store.tree();
    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("plan: nothing is written"), "{out}");
    assert!(out.contains("the first 1 whole block(s)"), "{out}");
    assert!(
        out.contains(
            "given a read-again line: 2020-03-02; not done, read by any Resume: 2020-03-03"
        ),
        "{out}"
    );
    assert_eq!(store.tree(), tree, "the plan writes nothing");

    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("Recovered; the original month's bytes are unchanged"),
        "{out}"
    );
    // THE IMPORTER'S OWN READER takes the line, and no longer calls the day
    // done: a plain Resume reads it.
    let done = pull::gdfl_import::done_whole_days(&store.root).unwrap();
    assert!(!done.contains("indices 2020-03-02"), "{done:?}");
    assert!(!done.contains("indices 2020-03-03"), "{done:?}");
    let journal = fs::read_to_string(pull::gdfl_import::journal_path(&store.root)).unwrap();
    let last = journal.lines().last().unwrap();
    assert!(
        last.starts_with("incomplete indices 2020-03-02 * definition="),
        "{last}"
    );
    assert!(last.ends_with(&format!("stage=file reason={REREAD_REASON}")));
    // ITS REASON IS THE IMPORTER'S OWN WORD FOR A DAMAGED MONTH, AND NO HOLE:
    // `admit` reads a non-hole `incomplete` key on a plain Resume.
    assert_eq!(pull::ingest::FailKind::StoreDamaged.code(), REREAD_REASON);
    assert!(!pull::ingest::FailKind::StoreDamaged.is_hole());
    // THE END RECORD lists the days it dropped.
    let ledger = store.ledger();
    assert!(ledger.starts_with("recovered month=bars/gdfl/NSE/INDEX/NIFTY/1min/2020-03.bin "));
    assert!(
        ledger.contains(" days=2020-03-02..2020-03-03 reread=2020-03-02 "),
        "{ledger}"
    );
    let record = store::recover::read_record(&store.root, store.path())
        .unwrap()
        .unwrap();
    assert!(record.reread_named);

    // A SECOND YES writes nothing and says so.
    let tree = store.tree();
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("Already recovered; nothing was written"),
        "{out}"
    );
    assert_eq!(store.tree(), tree);

    // THE RESUME: both days read again; the month is the clean month.
    store.reread(&store.day(2, 100));
    store.reread(&store.day(3, 80));
    let whole: Vec<Bar> = store
        .day(2, 100)
        .into_iter()
        .chain(store.day(3, 80))
        .collect();
    assert_eq!(store.served_bars(), whole);
}

/// RA-330: A POWER CUT DURING A COMPACTION PLANS AND RESUMES.
///
/// Two faults at once: the census partition was full, so the day's publish
/// began a compaction, and the power was cut inside it — the next epoch's
/// `.tmp` half written, never renamed — while a month the day appended to
/// was left torn. The plan names the month's cut and leaves the census as
/// it is (the current epoch is still the old one; nobody reads a `.tmp`);
/// the recovery writes the month's revision; the resumed day's publish
/// compacts again over the dead `.tmp` and installs the next epoch, whose
/// row is the served month's header, and the old epoch keeps every byte.
#[test]
fn a_power_cut_during_a_compaction_plans_and_resumes() {
    let store = Store::new(Vendor::Gdfl);
    let key = EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: Symbol::new("NIFTY").unwrap(),
        timeframe: Timeframe::MINUTE_1,
        month: store.month,
    };
    let held = |rows: u64, first: i64, last: i64| {
        Held::new(
            Entry {
                key,
                rows,
                first_ts_micros: first,
                last_ts_micros: last,
            },
            Closes::UNKNOWN,
        )
    };
    let day_two = store.day(2, 100);
    let (first, last) = (day_two[0].ts_micros, day_two[99].ts_micros);
    // A FULL PARTITION: capacity two, the month's key written twice.
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Index, 2).unwrap();
    census.record_held(held(50, first, last)).unwrap();
    census.record_held(held(100, first, last)).unwrap();
    let epoch_one = epoch_path(&store.root, Vendor::Gdfl, Segment::Index, 1);
    fs::create_dir_all(epoch_one.parent().unwrap()).unwrap();
    fs::write(&epoch_one, census.image()).unwrap();
    let old_epoch = fs::read(&epoch_one).unwrap();
    // THE CUT COMPACTION: its next epoch half written, never renamed.
    let dead_tmp = PathBuf::from(format!(
        "{}.tmp",
        epoch_path(&store.root, Vendor::Gdfl, Segment::Index, 2).display()
    ));
    fs::write(&dead_tmp, [0x5A; 4_096]).unwrap();
    store.tear(false);
    store.journal(JOURNAL);
    let current = || {
        pull::ingest::census_store::current_epoch(&store.root, Vendor::Gdfl, Segment::Index)
            .unwrap()
    };
    assert_eq!(current(), Some(1), "a .tmp is never the current epoch");

    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("the commit of generation 1: keeps 100 bars"),
        "{out}"
    );
    assert_eq!(current(), Some(1));
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    // THE LAST KEPT BAR'S DAY IS READ AGAIN TOO: the bars past the cut are
    // unproven, so whether day two lost any cannot be told, and a re-read of
    // a whole day answers its held bars `AlreadyPresent`.
    assert!(
        out.contains(
            "given a read-again line: 2020-03-02; not done, read by any Resume: 2020-03-03"
        ),
        "{out}"
    );
    assert_eq!(
        fs::read(&epoch_one).unwrap(),
        old_epoch,
        "the census is the census's"
    );

    // THE RESUME: the open day lands as the month's successor, and its census
    // publish compacts the full partition again, over the dead `.tmp`.
    store.reread(&store.day(3, 80));
    let served = BarFile::open_existing(&store.root, store.path(), Store::symbol_id())
        .unwrap()
        .header();
    assert_eq!(served.n_valid, 180);
    let target =
        pull::ingest::census_store::target_for(&store.root, Vendor::Gdfl, Segment::Index).unwrap();
    let cycle = pull::ingest::census_store::cycle(
        &target,
        &[held(
            served.n_valid,
            served.first_ts_micros,
            served.last_ts_micros,
        )],
        pull::ingest::census_store::Mode::AllOrNothing,
    )
    .unwrap();
    assert!(cycle.compacted.is_some(), "the full partition compacted");
    assert_eq!(current(), Some(2), "the next epoch is installed");
    assert_eq!(
        fs::read(&epoch_one).unwrap(),
        old_epoch,
        "the old epoch keeps every byte"
    );
    let now = pull::ingest::census_store::read_epoch(
        &epoch_path(&store.root, Vendor::Gdfl, Segment::Index, 2),
        Vendor::Gdfl,
        Segment::Index,
    )
    .unwrap();
    let rows: Vec<u64> = now
        .newest()
        .into_iter()
        .filter(|entry| entry.key == key)
        .map(|entry| entry.rows)
        .collect();
    assert_eq!(
        rows,
        vec![180],
        "the census row is the served month's header"
    );
}

/// MISUSE: every malformed argument list is refused by name, with the usage,
/// before the store root is even asked for.
#[test]
fn every_misuse_is_refused_before_the_root_is_asked_for() {
    let asked = std::cell::Cell::new(false);
    let root = || -> Result<PathBuf, String> {
        asked.set(true);
        Ok(PathBuf::from("/nowhere"))
    };
    for (arguments, says) in [
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1min", "2020-03"][..],
            "takes five arguments",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1min", "2020-03", "--apply"],
            "is not --plan or --yes",
        ),
        (
            &["groww2", "NSE/INDEX/NIFTY", "1min", "2020-03", "--plan"],
            "is not a feed",
        ),
        (
            &["gdfl", "NSE/INDEX", "1min", "2020-03", "--plan"],
            "is not EXCHANGE/SEGMENT/SYMBOL",
        ),
        (
            &["gdfl", "NSE//NIFTY", "1min", "2020-03", "--plan"],
            "has an empty segment",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "7min", "2020-03", "--plan"],
            "is not a rung",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1min", "2020-13", "--plan"],
            "2020-13",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1min", "1969-12", "--plan"],
            "1969-12",
        ),
        (
            &["gdfl", "NSE/IN DEX/NIFTY", "1min", "2020-03", "--plan"],
            "is not a store path",
        ),
    ] {
        let mut out = String::new();
        assert_eq!(command_with(arguments, &root, &mut out), MISUSED, "{out}");
        assert!(out.contains(says), "{arguments:?}: {out}");
        assert!(out.contains("usage: cli month-recover"), "{out}");
    }
    assert!(!asked.get(), "the root was asked for by a misuse");
    let mut out = String::new();
    let refused = || -> Result<PathBuf, String> { Err("no store root".to_owned()) };
    assert_eq!(
        command_with(
            &[
                "gdfl",
                "NSE/FNO/NIFTY/2020-03-26-900000-CE",
                "1min",
                "2020-03",
                "--plan"
            ],
            &refused,
            &mut out
        ),
        FAILED
    );
    assert!(
        out.contains("no store root. Nothing was read or written."),
        "{out}"
    );
}

/// A REFUSED MONTH, A CLEAN ONE, ANOTHER FEED: a month that is not there is
/// refused by name in both forms; a clean month is answered and left alone;
/// a Zerodha month keeps no import journal, so its days are named for its
/// own re-pull and no journal is written.
#[test]
fn refusals_clean_months_and_another_feed_are_each_said_by_name() {
    let store = Store::new(Vendor::Gdfl);
    for form in ["--plan", "--yes"] {
        let (code, out) = store.run(form);
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains("MONTH RECOVER REFUSED: month_missing"),
            "{out}"
        );
    }
    store.commit(&store.day(2, 100));
    let tree = store.tree();
    for form in ["--plan", "--yes"] {
        let (code, out) = store.run(form);
        assert_eq!(code, OK, "{out}");
        assert!(out.contains("every one of its 100 bars verifies"), "{out}");
    }
    assert_eq!(store.tree(), tree, "a clean month writes nothing");

    let zerodha = Store::new(Vendor::Zerodha);
    zerodha.tear(false);
    let (code, out) = zerodha.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("zerodha keeps no import journal"), "{out}");
    let (code, out) = zerodha.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(
        !zerodha.root.join("imports").exists(),
        "no journal for zerodha"
    );
    assert!(
        zerodha.ledger().contains(" reread=none "),
        "{}",
        zerodha.ledger()
    );
}

/// CRASH AND RESUME AT THE VERB: a recovery whose re-read could not be named
/// (the journal ends in a torn line) stands, the verb fails by name, and the
/// same command once the journal is whole writes exactly what was owed:
/// the line, the end record and the note, and then nothing more.
#[test]
fn a_re_read_not_named_is_written_by_the_next_run_and_only_once() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal(&format!("{JOURNAL}begin indices 2020-03-04"));
    let (code, out) = store.run("--yes");
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("ends in a torn line"), "{out}");
    let record = store::recover::read_record(&store.root, store.path())
        .unwrap()
        .unwrap();
    assert!(!record.reread_named, "recovered, the re-read not named");
    assert_eq!(store.ledger(), "", "no end record yet");

    store.journal(JOURNAL);
    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("is NOT named yet"), "{out}");
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("given a read-again line: 2020-03-02"), "{out}");
    assert_eq!(store.ledger().lines().count(), 1);
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("Already recovered; nothing was written"),
        "{out}"
    );
    let journal = fs::read_to_string(pull::gdfl_import::journal_path(&store.root)).unwrap();
    assert_eq!(
        journal
            .lines()
            .filter(|line| line.starts_with("incomplete indices 2020-03-02"))
            .count(),
        1,
        "{journal}"
    );
}

/// TWO WRITERS AT ONCE: a GDFL run holding its lock keeps the re-read lines
/// waiting (named), and a journal that cannot be read is named in the plan.
#[test]
fn a_live_run_or_an_unreadable_journal_is_named() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal("forgotten indices 2020-03-02 *\n");
    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("the import journal cannot be read"), "{out}");
    let (code, out) = store.run("--yes");
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("the re-read was not named"), "{out}");

    store.journal(JOURNAL);
    // THE GDFL STORE LOCK, held as a running import holds it.
    let lock = pull::gdfl_run::locks::lock_names(&store.root)
        .into_iter()
        .next()
        .unwrap();
    fs::create_dir_all(store.root.join(&lock).parent().unwrap()).unwrap();
    let held = pull::gdfl_run::locks::take_path_within(
        &store.root.join(&lock),
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let (code, out) = store.run("--yes");
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("run_busy") || out.contains("run_lock_refused"),
        "{out}"
    );
    drop(held);
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
}

/// The paths the verb names are the store's own.
#[test]
fn the_ledger_is_the_vendors_recoveries_file() {
    assert_eq!(LEDGER, "recoveries");
    let line = super::reread_line(pull::gdfl_import::ImportKind::Options, "2019-07-01");
    assert!(line.starts_with("incomplete options 2019-07-01 * definition="));
    assert!(
        Path::new(&format!("audit/gdfl.{LEDGER}"))
            .extension()
            .is_some()
    );
}

/// A SEGMENT NO IMPORT KIND FILES: a GDFL month outside every import kind's
/// segment has no journal to name its days in. The plan and the recovery
/// say so by name and name the days for a re-read by hand; no journal is
/// written, and the end record lists no read-again line.
#[test]
fn a_segment_no_import_kind_files_names_its_days_for_a_re_read_by_hand() {
    let mut store = Store::new(Vendor::Gdfl);
    store.segment = "BOGUS";
    store.tear(false);
    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("no GDFL import kind files the segment BOGUS"),
        "{out}"
    );
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(!store.root.join("imports").exists(), "no journal written");
    assert!(
        store.ledger().contains(" reread=none "),
        "{}",
        store.ledger()
    );
}

/// CRASH BETWEEN THE PROMOTION AND THE RECORD, at the verb: the record file
/// lost (a door cut off before it). The plan says the revision is served
/// and only the record is owed; the yes writes the record, says it found
/// the revision served, and writes the end record again.
#[test]
fn a_door_cut_off_before_its_record_is_planned_and_resumed_by_name() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal(JOURNAL);
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    fs::remove_file(store::recover::record_path(&store.root, store.path())).unwrap();
    let (code, out) = store.run("--plan");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("promoted  revision 1 is served already"),
        "{out}"
    );
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("found served: a run cut off before its record"),
        "{out}"
    );
    assert_eq!(store.ledger().lines().count(), 2, "{}", store.ledger());
}

/// THE END RECORD CANNOT BE WRITTEN (a file holds the audit directory's
/// name): the recovery stands, the verb fails by name, and the same command
/// once the directory is back writes the end record and the note.
#[test]
fn an_end_record_that_cannot_be_written_fails_by_name_and_is_written_next_run() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal(JOURNAL);
    fs::write(store.root.join("audit"), b"not a directory").unwrap();
    let (code, out) = store.run("--yes");
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("the end record was not written"), "{out}");
    assert!(out.contains("the same command writes it"), "{out}");
    fs::remove_file(store.root.join("audit")).unwrap();
    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    assert_eq!(store.ledger().lines().count(), 1);
}

/// THE STEPS' OWN REFUSALS, each by name: the journal's lock held past the
/// wait, a journal that cannot be opened, a note against a recovery that is
/// not the month's, a contract segment the store does not render, the
/// receipt of a view with no day on either side, and the env door's misuse.
#[test]
fn every_step_refusal_is_said_by_name() {
    let store = Store::new(Vendor::Gdfl);
    store.tear(true);
    store.journal(JOURNAL);
    let arguments = ["gdfl", "NSE/INDEX/NIFTY", "1min", "2020-03", "--yes"];
    let ask = super::ask(&arguments).unwrap();
    let store::recover::Plan::Cut { dropped, .. } =
        store::recover::plan(&store.root, store.path(), Store::symbol_id()).unwrap()
    else {
        panic!("cut");
    };
    let journal = pull::gdfl_import::journal_path(&store.root);
    let lock = pull::gdfl_import::journal_lock_path(&journal);
    let held =
        pull::gdfl_run::locks::take_path_within(&lock, std::time::Duration::from_secs(1)).unwrap();
    let waited =
        super::write_reread(&store.root, &ask, &dropped, std::time::Duration::ZERO).unwrap_err();
    assert!(waited.contains(&lock.display().to_string()), "{waited}");
    drop(held);

    let gone = store.root.join("no-such-directory").join("journal");
    let refused = super::append_lines(&gone, &["x".to_owned()]).unwrap_err();
    assert!(refused.contains("no-such-directory"), "{refused}");
    let empty = store.root.join("empty.journal");
    fs::write(&empty, b"").unwrap();
    super::append_lines(&empty, &["a whole line".to_owned()]).unwrap();
    assert_eq!(fs::read_to_string(&empty).unwrap(), "a whole line\n");

    let (code, out) = store.run("--yes");
    assert_eq!(code, OK, "{out}");
    let record = store::recover::read_record(&store.root, store.path())
        .unwrap()
        .unwrap();
    let foreign = store::recover::Record {
        revision: store::repair::Revision::new(7).unwrap(),
        ..record
    };
    let refused = super::noted(&store.root, store.path(), &foreign).unwrap_err();
    assert!(refused.contains("the re-read was not noted"), "{refused}");

    let mut out = String::new();
    assert_eq!(
        command_with(
            &[
                "gdfl",
                "NSE/FNO/NIFTY/not-a-contract",
                "1min",
                "2020-03",
                "--plan"
            ],
            &|| Ok(PathBuf::from("/nowhere")),
            &mut out
        ),
        MISUSED
    );
    assert!(out.contains("is not a contract segment"), "{out}");

    let mut out = String::new();
    super::render_view(&mut out, &super::Reread::default());
    assert!(
        out.contains("given a read-again line: none; not done, read by any Resume: none"),
        "{out}"
    );

    let mut out = String::new();
    assert_eq!(super::command(&["gdfl"], &mut out), MISUSED, "{out}");
}
