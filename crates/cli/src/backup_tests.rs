//! `cli backup` and `cli backup-drill` on owned stores of invented bars
//! (R4-09b, AUT-06; D-7137, D-7175..D-7181). Prices are invented and cannot
//! be mistaken for real ones; every root is a private temporary folder.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Closes, EntryKey, Held as CensusHeld};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{Timeframe, YearMonth};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A private store root, a target on "another device" and a scratch parent.
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    target: PathBuf,
    scratch: PathBuf,
}

fn fixture(tag: &str) -> Fixture {
    let base = std::env::temp_dir().join(format!("brutex-cli-backup-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    for dir in ["store/manifest", "target", "scratch", "archives"] {
        fs::create_dir_all(base.join(dir)).unwrap();
    }
    let base = fs::canonicalize(&base).unwrap();
    Fixture {
        root: base.join("store"),
        target: base.join("target"),
        scratch: base.join("scratch"),
        base,
    }
}

/// The three kinds the drill must span: an index month, a share month of ONE
/// bar, and an option month.
const KINDS: [(&str, &str, bool, u8, i64); 3] = [
    ("INDEX", "NIFTY", false, 1, 150),
    ("CASH", "ZZSHARE", false, 2, 1),
    ("FNO", "NIFTY", true, 3, 100),
];

fn contract() -> Contract {
    Contract::parse("2019-12-26-1200000-CE").expect("a contract")
}

fn segment_of(name: &str) -> Segment {
    match name {
        "INDEX" => Segment::Index,
        "CASH" => Segment::Cash,
        _ => Segment::Fno,
    }
}

#[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
fn symbol_id(symbol: &str) -> u32 {
    brutex_core::universe::fnv1a(symbol) as u32
}

fn month_path(segment: &str, symbol: &'static str, option: bool, m: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: match segment {
            "INDEX" => "INDEX",
            "CASH" => "CASH",
            _ => "FNO",
        },
        symbol,
        contract: option.then(contract),
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, m).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `n` invented minute bars of 2025-`m`, from minute `from` of day 2.
fn bars(m: u8, from: i64, n: i64) -> Vec<Bar> {
    let start = i64::from(
        pull::session::Day::new(2025, m, 2)
            .unwrap()
            .days_from_epoch(),
    ) * 86_400_000_000;
    (from..from + n)
        .map(|i| Bar {
            ts_micros: start + i * 60_000_000,
            open: 333_333 + i,
            high: 333_433 + i,
            low: 333_233 + i,
            close: 333_333 + i,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect()
}

impl Fixture {
    /// The three kinds of month, written and counted in the census, plus a
    /// ledger, an empty ops file, and a journal naming an archive zip and a
    /// tick store.
    fn standard(tag: &str) -> Self {
        let fx = fixture(tag);
        let mut held = Vec::new();
        for (segment, symbol, option, m, n) in KINDS {
            let path = month_path(segment, symbol, option, m);
            let batch = bars(m, 0, n);
            let mut writer = BarFile::open_or_create(&fx.root, path, symbol_id(symbol)).unwrap();
            writer.append(&batch).unwrap();
            held.push(CensusHeld::new(
                Entry {
                    key: EntryKey {
                        contract: option.then(contract),
                        exchange: Exchange::Nse,
                        segment: segment_of(segment),
                        symbol: Symbol::new(symbol).unwrap(),
                        timeframe: Timeframe::MINUTE_1,
                        month: YearMonth::new(2025, m).unwrap(),
                    },
                    rows: u64::try_from(n).unwrap(),
                    first_ts_micros: batch[0].ts_micros,
                    last_ts_micros: batch[batch.len() - 1].ts_micros,
                },
                Closes::UNKNOWN,
            ));
        }
        census_store::record(&fx.root, Vendor::Gdfl, &held).unwrap();
        fs::create_dir_all(fx.root.join("audit")).unwrap();
        fs::write(
            fx.root.join("audit/scrub-blocks.gdfl.log"),
            "run 1 invented\n",
        )
        .unwrap();
        fs::create_dir_all(fx.root.join("ops")).unwrap();
        fs::write(fx.root.join("ops/empty.log"), "").unwrap();
        fs::write(fx.base.join("archives/outer.zip"), b"invented zip bytes").unwrap();
        fs::create_dir_all(fx.base.join("archives/ticks/2019")).unwrap();
        fs::write(
            fx.base.join("archives/ticks/2019/a.bts"),
            b"BRTXTS01 invented",
        )
        .unwrap();
        fs::write(fx.base.join("archives/ticks/FORMAT.md"), b"invented").unwrap();
        fx.journal(&[
            &fx.base.join("archives/outer.zip"),
            &fx.base.join("archives/ticks"),
        ]);
        fx
    }

    /// An import journal whose `run` lines name `archives`.
    fn journal(&self, archives: &[&Path]) {
        let mut text = String::new();
        for (n, archive) in archives.iter().enumerate() {
            let _ = writeln!(
                text,
                "run {} source=zip archive={} archive_bytes=1 commit=unstamped started=1",
                1_700_000_000 + n,
                encoded(archive.as_os_str().as_bytes())
            );
        }
        text.push_str("run 1800000000 source=unrecorded archive=unrecorded archive_bytes=unrecorded commit=unrecorded started=1\n");
        let path = pull::gdfl_import::journal_path(&self.root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn device(&self) -> impl Fn(&Path) -> io::Result<u64> + use<> {
        let target = self.target.clone();
        move |path: &Path| Ok(if path.starts_with(&target) { 7 } else { 9 })
    }

    /// `backup` asked directly, the target on another device and `free` bytes
    /// free there.
    fn backup_with(&self, plan: bool, inject: Inject, free: u64) -> Result<Ran, Refusal> {
        let device = self.device();
        let free = move |_: &Path| -> Result<u64, String> { Ok(free) };
        let env = Env {
            commit: Some(COMMIT),
            device: &device,
            free: &free,
            inject,
        };
        let ask = Ask {
            to: self.target.clone(),
            plan,
            bytes_per_s: 0,
        };
        backup(&self.root, &ask, &env)
    }

    fn backup(&self) -> Ran {
        self.backup_with(false, Inject::default(), u64::MAX)
            .expect("a backup")
    }

    /// The verb with `arguments`.
    fn verb(&self, arguments: &[&str]) -> (u8, String) {
        let device = self.device();
        let free = |_: &Path| -> Result<u64, String> { Ok(u64::MAX) };
        let env = Env {
            commit: Some(COMMIT),
            device: &device,
            free: &free,
            inject: Inject::default(),
        };
        let root = self.root.clone();
        let store = move || Ok(root.clone());
        let mut out = String::new();
        let code = command_with(arguments, &store, &env, &mut out);
        (code, out)
    }

    fn to(&self) -> String {
        self.target.display().to_string()
    }

    /// The drill verb with `arguments`, restoring under the fixture's scratch.
    fn drill(&self, arguments: &[&str]) -> (u8, String) {
        let root = self.root.clone();
        let store = move || Ok(root.clone());
        let mut out = String::new();
        let code = drill_command_with(arguments, &store, &self.scratch, Some(COMMIT), &mut out);
        (code, out)
    }

    fn manifest(&self) -> String {
        fs::read_to_string(self.target.join(HOME).join(MANIFEST)).unwrap_or_default()
    }

    fn records(&self) -> Vec<Record> {
        self.manifest().lines().filter_map(record_of).collect()
    }

    fn receipts(&self) -> Vec<String> {
        fs::read_to_string(self.root.join(RECEIPTS))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Every file under the target's run folders.
    fn copies(&self) -> Vec<PathBuf> {
        let mut faults = Faults::default();
        walk(
            &self.target.join(HOME).join(RUNS),
            "x",
            None,
            &[],
            &mut faults,
        )
        .into_iter()
        .map(|found| found.source)
        .collect()
    }

    /// Where `record`'s bytes are.
    fn copy_of(&self, record: &Record) -> PathBuf {
        let path = Path::new(std::ffi::OsStr::from_bytes(&record.path));
        let mut out = self
            .target
            .join(HOME)
            .join(RUNS)
            .join(record.run.to_string())
            .join(if record.class == ARCHIVE {
                "archives"
            } else {
                "store"
            });
        for part in path.components() {
            if let Component::Normal(part) = part {
                out.push(part);
            }
        }
        out
    }

    /// Every record's copy exists, holds its length, and hashes to its digest.
    fn every_record_verifies(&self) {
        for record in self.records() {
            let bytes = fs::read(self.copy_of(&record)).unwrap_or_else(|why| {
                panic!("{}: {why}", shown(&record.path));
            });
            assert_eq!(bytes.len() as u64, record.len, "{}", shown(&record.path));
            assert_eq!(
                brutex_core::blake3::hash(&bytes),
                record.digest,
                "{}",
                shown(&record.path)
            );
        }
    }

    fn bin(&self, segment: &str, symbol: &'static str, option: bool, m: u8) -> String {
        month_path(segment, symbol, option, m).to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn words_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(' ')
        .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
}

// ------------------------------------------------------------ the spec's tests

/// **A SECOND BACKUP COPIES NOTHING (AUT-06).** The first run copies every
/// listed file — the three months' siblings, the census, the journal, the
/// ledger, an empty ops file and both archives — and appends one receipt; the
/// second copies none, writes no file on the target, and appends a receipt
/// that says `copied_files=0`. Its own receipt log is not listed, so the run
/// that wrote it does not make the next one copy it.
#[test]
fn a_second_backup_copies_nothing() {
    let fx = Fixture::standard("second");
    let first = fx.backup();
    let total = first.total();
    assert!(first.complete(), "{first:?}");
    assert_eq!(total.copied, total.listed, "{first:?}");
    assert!(
        total.copied >= 10,
        "every sibling, census, journal, archive: {first:?}"
    );
    for class in [MONTH, "census", "journal", "ledger", "ops", ARCHIVE] {
        assert!(first.classes[class].copied > 0, "{class}: {first:?}");
    }
    // AN EMPTY FILE IS A FILE: copied, recorded with length 0.
    let empty = fx
        .records()
        .into_iter()
        .find(|r| r.path == b"ops/empty.log")
        .expect("the empty ops file");
    assert_eq!(empty.len, 0);
    assert_eq!(empty.digest, brutex_core::blake3::hash(&[]));
    fx.every_record_verifies();
    let copies = fx.copies().len();
    assert_eq!(copies as u64, total.copied);
    let receipts = fx.receipts();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].starts_with("copied=20"), "{}", receipts[0]);
    assert_eq!(
        words_of(&receipts[0], "dest").and_then(backup_decoded_str),
        Some(fx.to())
    );
    assert_eq!(
        words_of(&receipts[0], "copied_files"),
        Some(total.copied.to_string().as_str())
    );

    let second = fx.backup();
    assert_eq!(second.total().copied, 0, "{second:?}");
    assert_eq!(second.total().unchanged, total.listed);
    assert_eq!(
        fx.copies().len(),
        copies,
        "no file was written on the target"
    );
    let receipts = fx.receipts();
    assert_eq!(receipts.len(), 2);
    assert!(receipts[1].contains(" state=complete "), "{}", receipts[1]);
    assert_eq!(words_of(&receipts[1], "copied_files"), Some("0"));
    assert_eq!(
        words_of(&receipts[1], "files"),
        Some(total.listed.to_string().as_str()),
        "the target still holds every file"
    );
    assert!(second.run > first.run, "run ids only grow");
    let manifest = fx.manifest();
    let after = manifest
        .split(&format!("run {} ", second.run))
        .nth(1)
        .unwrap();
    assert!(
        !after.contains("\nfile "),
        "the second run recorded no copy: {after}"
    );
}

fn backup_decoded_str(value: &str) -> Option<String> {
    String::from_utf8(decoded(value)?).ok()
}

/// **A CHANGED FILE GETS A NEW COPY AND THE OLD COPY STAYS (D-7176).** Bars
/// appended to the index month change its files; the next run copies them
/// under its own run folder, the first run's copies are byte for byte what
/// they were, and the unchanged share month is not copied again.
#[test]
fn a_changed_file_gets_a_new_copy_and_the_old_copy_stays() {
    let fx = Fixture::standard("changed");
    let first = fx.backup();
    let bin = fx.bin("INDEX", "NIFTY", false, 1);
    let old = fx
        .records()
        .into_iter()
        .find(|r| r.path == bin.as_bytes())
        .unwrap();
    let old_bytes = fs::read(fx.copy_of(&old)).unwrap();

    let mut writer = BarFile::open_or_create(
        &fx.root,
        month_path("INDEX", "NIFTY", false, 1),
        symbol_id("NIFTY"),
    )
    .unwrap();
    writer.append(&bars(1, 150, 10)).unwrap();
    drop(writer);
    let second = fx.backup();
    assert!(second.classes[MONTH].copied > 0, "{second:?}");
    let newest = fx
        .records()
        .into_iter()
        .rev()
        .find(|r| r.path == bin.as_bytes())
        .unwrap();
    assert_eq!(newest.run, second.run);
    assert_ne!(newest.digest, old.digest);
    assert_eq!(
        fs::read(fx.copy_of(&newest)).unwrap(),
        fs::read(fx.root.join(&bin)).unwrap(),
        "the new copy is the store's new bytes"
    );
    assert_eq!(
        fs::read(fx.copy_of(&old)).unwrap(),
        old_bytes,
        "the first run's copy is kept, never overwritten"
    );
    assert!(fx.copy_of(&old) != fx.copy_of(&newest));
    let share = fx.bin("CASH", "ZZSHARE", false, 2);
    assert!(
        !fx.copies().iter().any(|copy| copy.starts_with(
            fx.target
                .join(HOME)
                .join(RUNS)
                .join(second.run.to_string())
                .join("store")
                .join(&share)
                .parent()
                .unwrap()
        )),
        "the unchanged share month was not copied again"
    );
    assert!(first.run < second.run);
    fx.every_record_verifies();
}

/// **A TARGET ON THE STORE'S DEVICE IS REFUSED (D-7137).** The machine's own
/// `st_dev` answers: both folders are on one filesystem here. Nothing is
/// written on either side.
#[test]
fn a_target_on_the_store_device_is_refused() {
    let fx = Fixture::standard("same-device");
    let free = |_: &Path| -> Result<u64, String> { Ok(u64::MAX) };
    let env = Env {
        commit: Some(COMMIT),
        device: &super::device,
        free: &free,
        inject: Inject::default(),
    };
    let root = fx.root.clone();
    let store = move || Ok(root.clone());
    let mut out = String::new();
    let code = command_with(&["--to", &fx.to()], &store, &env, &mut out);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("BACKUP REFUSED (same_device)") && out.contains("st_dev"),
        "{out}"
    );
    assert_eq!(
        fs::read_dir(&fx.target).unwrap().count(),
        0,
        "nothing on the target"
    );
    assert!(fx.receipts().is_empty(), "no receipt");
}

/// **A TARGET INSIDE THE STORE IS REFUSED (D-7137)**, before anything is
/// written, whatever its device.
#[test]
fn a_target_inside_the_store_is_refused() {
    let fx = Fixture::standard("inside");
    let inside = fx.root.join("audit/copy");
    fs::create_dir_all(&inside).unwrap();
    let (code, out) = fx.verb(&["--to", &inside.display().to_string()]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("BACKUP REFUSED (inside_store)"), "{out}");
    assert_eq!(fs::read_dir(&inside).unwrap().count(), 0);
    assert!(fx.receipts().is_empty());
}

/// **A KILL MID-COPY RESUMES WITHOUT A TORN FILE (D-7176).** A run cut half
/// way through a copy, with a torn record behind it, leaves a temporary name
/// and no record of the cut file; no receipt. The next run ends the torn tail
/// as a void line, copies everything the manifest does not call done, and
/// every record names a whole copy whose digest is its bytes'. The temporary
/// is left where it was, unnamed by any record.
#[test]
fn a_kill_mid_copy_resumes_without_a_torn_file() {
    let fx = Fixture::standard("kill");
    let killed = fx
        .backup_with(
            false,
            Inject {
                kill_after: Some(2),
                ..Inject::default()
            },
            u64::MAX,
        )
        .expect_err("killed");
    assert_eq!(killed.word, "killed");
    let manifest = fx.manifest();
    assert!(!manifest.ends_with('\n'), "a torn record: {manifest:?}");
    assert!(fx.receipts().is_empty(), "a cut run appends no receipt");
    let temporaries: Vec<PathBuf> = fx
        .copies()
        .into_iter()
        .filter(|copy| copy.extension().is_some_and(|e| e == "tmp"))
        .collect();
    assert_eq!(temporaries.len(), 1, "{:?}", fx.copies());
    let torn = fs::read(&temporaries[0]).unwrap();

    let resumed = fx.backup();
    assert!(resumed.complete(), "{resumed:?}");
    assert!(
        fx.manifest().contains(&format!("{VOID}\n")),
        "the torn tail ended"
    );
    fx.every_record_verifies();
    assert_eq!(
        fx.records().len() as u64,
        resumed.total().listed,
        "every listed file is recorded once its copy is whole"
    );
    assert!(
        fx.records().iter().all(|r| !r.path.ends_with(b".tmp")),
        "no record names a temporary"
    );
    assert_eq!(
        fs::read(&temporaries[0]).unwrap(),
        torn,
        "the temporary is left alone"
    );
    assert_eq!(fx.receipts().len(), 1);
}

/// **A WRITER-HELD MONTH IS BUSY AND RETRIED (D-7137).** While a writer holds
/// the option month, the run copies everything else, names the month on a
/// `busy` line and in its receipt (`state=partial busy=1`) and exits FAILED;
/// none of the month's files is copied half-written. Released, the next run
/// copies it and the receipt says `complete`.
#[test]
fn a_writer_held_month_is_busy_and_retried() {
    let fx = Fixture::standard("busy");
    let unit = fx.bin("FNO", "NIFTY", true, 3);
    let writer = BarFile::open_or_create(
        &fx.root,
        month_path("FNO", "NIFTY", true, 3),
        symbol_id("NIFTY"),
    )
    .unwrap();
    let (code, out) = fx.verb(&["--to", &fx.to()]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("BUSY") && out.contains(&unit), "{out}");
    assert!(out.contains("PARTIAL"), "{out}");
    assert!(
        fx.manifest()
            .contains(&format!("busy {} run=", encoded(unit.as_bytes()))),
        "{}",
        fx.manifest()
    );
    assert!(
        !fx.records()
            .iter()
            .any(|r| r.unit.as_deref() == Some(unit.as_bytes())),
        "no file of the held month was copied"
    );
    let receipt = fx.receipts().pop().unwrap();
    assert!(
        receipt.contains(" state=partial ") && receipt.contains(" busy=1 "),
        "{receipt}"
    );
    drop(writer);

    let (code, out) = fx.verb(&["--to", &fx.to()]);
    assert_eq!(code, OK, "{out}");
    assert!(
        fx.records()
            .iter()
            .any(|r| r.unit.as_deref() == Some(unit.as_bytes())),
        "the month is copied once released"
    );
    let receipt = fx.receipts().pop().unwrap();
    assert!(
        receipt.contains(" state=complete ") && receipt.contains(" busy=0 "),
        "{receipt}"
    );
    fx.every_record_verifies();
}

/// **THE DRILL RESTORES THREE MONTHS BYTE-EQUAL (D-7137).** Three months, one
/// of each kind, restored into a new scratch folder outside the store and the
/// target, each file equal to its manifest digest and to the store's bytes;
/// one drill line `result=pass months=3`; the store's bars untouched.
#[test]
fn the_drill_restores_three_months_byte_equal() {
    let fx = Fixture::standard("drill");
    fx.backup();
    let before: Vec<Vec<u8>> = KINDS
        .iter()
        .map(|(s, y, o, m, _)| fs::read(fx.root.join(fx.bin(s, *y, *o, *m))).unwrap())
        .collect();
    let (code, out) = fx.drill(&["--from", &fx.to()]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains(": PASS: 3 of 3 month(s) restored"), "{out}");
    for (segment, symbol, option, m, _) in KINDS {
        let bin = fx.bin(segment, symbol, option, m);
        assert!(out.contains(&bin), "{bin} restored: {out}");
    }
    let line = fs::read_to_string(fx.root.join(DRILLS)).unwrap();
    assert_eq!(line.lines().count(), 1);
    assert!(line.starts_with("drilled=20"), "{line}");
    assert_eq!(words_of(&line, "result"), Some("pass"));
    assert_eq!(words_of(&line, "months"), Some("3"));
    // EVERY RESTORED FILE IS THE STORE'S, byte for byte.
    let scratch: Vec<PathBuf> = fs::read_dir(&fx.scratch)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(scratch.len(), 1, "one new scratch folder");
    let mut faults = Faults::default();
    let restored = walk(&scratch[0], "x", Some(&scratch[0]), &[], &mut faults);
    assert!(restored.len() >= 6, "{restored:?}");
    for file in restored {
        let rel = Path::new(std::ffi::OsStr::from_bytes(&file.path));
        assert_eq!(
            fs::read(&file.source).unwrap(),
            fs::read(fx.root.join(rel)).unwrap(),
            "{}",
            rel.display()
        );
    }
    let after: Vec<Vec<u8>> = KINDS
        .iter()
        .map(|(s, y, o, m, _)| fs::read(fx.root.join(fx.bin(s, *y, *o, *m))).unwrap())
        .collect();
    assert_eq!(
        before, after,
        "the drill wrote nothing into the store's bars"
    );
}

// ------------------------------------------------------------ extreme cases

/// **AN EMPTY STORE IS REFUSED, NOT BACKED UP AS "0 FILES".** A root with no
/// census, no listed file and no journal is the wrong folder: a receipt would
/// say a copy exists.
#[test]
fn an_empty_store_is_refused_and_nothing_is_written() {
    let fx = fixture("empty");
    let (code, out) = fx.verb(&["--to", &fx.to()]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("BACKUP REFUSED (nothing_listed)"), "{out}");
    assert_eq!(fs::read_dir(&fx.target).unwrap().count(), 0);
    assert!(fx.receipts().is_empty());
}

/// **GARBAGE PARAMETERS ARE MISUSE**, said with the usage, for both verbs.
#[test]
fn garbage_parameters_are_misuse() {
    let fx = fixture("garbage");
    for arguments in [
        &["--plan"][..],
        &["--to"],
        &["--to", ""],
        &["--to", "a", "--to", "b"],
        &["--to", "a", "--plan", "--plan"],
        &["--to", "a", "--mib-per-s", "fast"],
        &["--to", "a", "--mib-per-s", "-1"],
        &["--to", "a", "--mib-per-s"],
        &["--to", "a", "--mib-per-s", "1", "--mib-per-s", "2"],
        &["--to", "a", "--delete"],
    ] {
        let (code, out) = fx.verb(arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(
            out.starts_with("refused: ") && out.contains("backup takes"),
            "{out}"
        );
    }
    for arguments in [
        &["--months", "3"][..],
        &["--from"],
        &["--from", ""],
        &["--from", "a", "--months", "0"],
        &["--from", "a", "--months", "many"],
        &["--from", "a", "--months"],
        &["--from", "a", "--from", "b"],
        &["--from", "a", "--months", "1", "--months", "2"],
        &["--from", "a", "--plan"],
    ] {
        let (code, out) = fx.drill(arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(out.contains("backup-drill takes"), "{out}");
    }
    assert_eq!(parse_drill(&["--from", "x"]).unwrap().months, DRILL_MONTHS);
    assert_eq!(
        parse(&["--to", "x", "--mib-per-s", "2"])
            .unwrap()
            .bytes_per_s,
        2 * 1024 * 1024
    );
}

/// **A TARGET THAT IS A FILE, OR ABSENT, IS REFUSED BY NAME.**
#[test]
fn a_target_that_is_a_file_or_absent_is_refused_by_name() {
    let fx = Fixture::standard("not-dir");
    let file = fx.target.join("afile");
    fs::write(&file, "x").unwrap();
    let (code, out) = fx.verb(&["--to", &file.display().to_string()]);
    assert_eq!(code, FAILED);
    assert!(out.contains("BACKUP REFUSED (not_a_directory)"), "{out}");
    let absent = fx.base.join("unplugged");
    let (code, out) = fx.verb(&["--to", &absent.display().to_string()]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("BACKUP REFUSED (target_missing)") && out.contains("attached and mounted"),
        "{out}"
    );
    let (code, out) = fx.drill(&["--from", &absent.display().to_string()]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("BACKUP DRILL REFUSED (target_missing)"),
        "{out}"
    );
    let (code, out) = fx.drill(&["--from", &file.display().to_string()]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("BACKUP DRILL REFUSED (not_a_directory)"),
        "{out}"
    );
    assert!(fx.receipts().is_empty());
}

/// **A MANIFEST WITH A LINE THIS BUILD CANNOT READ STOPS THE RUN**: what the
/// target holds cannot be told, so nothing is copied and no receipt says
/// otherwise. A manifest some other tool wrote is refused the same way.
#[test]
fn a_manifest_with_a_foreign_line_refuses_and_copies_nothing() {
    let fx = Fixture::standard("foreign");
    fs::create_dir_all(fx.target.join(HOME)).unwrap();
    let manifest = fx.target.join(HOME).join(MANIFEST);
    for text in [
        format!("{MANIFEST_HEAD}\nrsync: something odd\n"),
        format!("{MANIFEST_HEAD}\nfile run=1 class=ops len=x mtime=0 blake3=00 path=ops/a\n"),
        "some other tool's file\n".to_owned(),
    ] {
        fs::write(&manifest, &text).unwrap();
        let refused = fx
            .backup_with(false, Inject::default(), u64::MAX)
            .expect_err("refused");
        assert_eq!(refused.word, "manifest_unreadable", "{text:?}: {refused:?}");
        assert!(!fx.target.join(HOME).join(RUNS).exists());
        assert!(fx.receipts().is_empty());
        assert_eq!(fs::read_to_string(&manifest).unwrap(), text, "untouched");
    }
}

/// **A LINK OR A FIFO IN A LISTED FOLDER IS SKIPPED BY NAME**, never followed:
/// a link to a folder outside the store would copy that folder.
#[test]
fn a_link_in_a_listed_folder_is_skipped_by_name() {
    let fx = Fixture::standard("link");
    fs::create_dir_all(fx.base.join("elsewhere")).unwrap();
    fs::write(fx.base.join("elsewhere/secret"), "not the store's").unwrap();
    std::os::unix::fs::symlink(fx.base.join("elsewhere"), fx.root.join("audit/away")).unwrap();
    let (code, out) = fx.verb(&["--to", &fx.to()]);
    assert_eq!(code, OK, "a skipped link is said, not a fault: {out}");
    assert!(
        out.contains("SKIPPED") && out.contains("audit/away (link)"),
        "{out}"
    );
    assert!(
        !fx.records()
            .iter()
            .any(|r| r.path.starts_with(b"audit/away"))
    );
}

/// **A MISSING ARCHIVE IS NAMED AND THE RUN IS PARTIAL**: the journal names a
/// zip that is no longer on disk (moved, renamed, the drive unplugged).
#[test]
fn a_missing_archive_is_named_and_the_run_is_partial() {
    let fx = Fixture::standard("no-archive");
    let gone = fx.base.join("archives/renamed.zip");
    fx.journal(&[&gone]);
    let (code, out) = fx.verb(&["--to", &fx.to()]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("MISSING") && out.contains("renamed.zip (archive)"),
        "{out}"
    );
    assert!(
        fx.receipts()[0].contains(" missing=1 "),
        "{:?}",
        fx.receipts()
    );
}

/// **AN OLDER GENERATION AND ITS POINTER ARE COPIED, AND RESTORED**: a repair
/// promotes revision 1 of the index month; the backup copies the original,
/// the revision's files with its receipt, and the pointer, all under the
/// month's unit; the drill restores and compares every one.
#[test]
fn an_older_generation_and_its_pointer_are_copied() {
    let fx = Fixture::standard("generation");
    let path = month_path("INDEX", "NIFTY", false, 1);
    let mut offered = bars(1, -5, 5);
    offered.extend(bars(1, 0, 150));
    store::repair::repair_month(
        &fx.root,
        path,
        symbol_id("NIFTY"),
        &offered,
        store::repair::Policy {
            open_interest_applies: false,
            basis_witness: false,
        },
    )
    .unwrap();
    fx.backup();
    let unit = path.to_string();
    let month: Vec<String> = fx
        .records()
        .into_iter()
        .filter(|r| r.unit.as_deref() == Some(unit.as_bytes()))
        .map(|r| shown(&r.path))
        .collect();
    assert!(month.contains(&unit), "{month:?}");
    assert!(
        month
            .iter()
            .any(|p| p.starts_with("bar-revisions-v1/1/") && p.ends_with(".bin")),
        "{month:?}"
    );
    assert!(month.iter().any(|p| p.ends_with(".repair-v1")), "{month:?}");
    assert!(
        month
            .iter()
            .any(|p| p.starts_with("bar-generations-v1/") && p.ends_with(".gen-v1")),
        "{month:?}"
    );
    let (code, out) = fx.drill(&["--from", &fx.to(), "--months", "3"]);
    assert_eq!(code, OK, "{out}");
}

/// **TOO LITTLE SPACE IS REFUSED BEFORE A BYTE IS WRITTEN**, with both
/// numbers; the plan says the same.
#[test]
fn too_little_space_is_refused_before_a_byte_is_written() {
    let fx = Fixture::standard("space");
    for plan in [false, true] {
        let refused = fx
            .backup_with(plan, Inject::default(), 10)
            .expect_err("refused");
        assert_eq!(refused.word, "insufficient_space");
        assert!(refused.why.contains("10 byte(s) free"), "{}", refused.why);
    }
    assert_eq!(fs::read_dir(&fx.target).unwrap().count(), 0);
    assert!(fx.receipts().is_empty());
}

/// **ONE RUN AT A TIME PER TARGET**: a second run while the first holds the
/// target's lock is refused by name and copies nothing; a drill is refused
/// too, so it never reads a manifest mid-run.
#[test]
fn a_second_run_at_once_is_refused_target_busy() {
    let fx = Fixture::standard("busy-target");
    fx.backup();
    let lock = File::open(fx.target.join(HOME).join(LOCK)).unwrap();
    let held = Flock::try_lock(lock, fx.target.join(HOME).join(LOCK)).unwrap();
    let refused = fx
        .backup_with(false, Inject::default(), u64::MAX)
        .expect_err("refused");
    assert_eq!(refused.word, "target_busy");
    let (code, out) = fx.drill(&["--from", &fx.to()]);
    assert_eq!(code, FAILED);
    assert!(out.contains("BACKUP DRILL REFUSED (target_busy)"), "{out}");
    drop(held);
    assert_eq!(fx.receipts().len(), 1);
}

/// **A TOUCHED FILE IS RECORDED, NOT COPIED**: same bytes under a newer time
/// is a new record over the copy already held.
#[test]
fn a_touched_file_is_recorded_not_copied() {
    let fx = Fixture::standard("touched");
    fx.backup();
    let copies = fx.copies().len();
    let ledger = File::options()
        .write(true)
        .open(fx.root.join("audit/scrub-blocks.gdfl.log"))
        .unwrap();
    ledger
        .set_modified(SystemTime::now() + Duration::from_secs(5))
        .unwrap();
    let second = fx.backup();
    assert_eq!(second.classes["ledger"].recorded_only, 1, "{second:?}");
    assert_eq!(second.total().copied, 0);
    assert_eq!(fx.copies().len(), copies);
    let records: Vec<Record> = fx
        .records()
        .into_iter()
        .filter(|r| r.path == b"audit/scrub-blocks.gdfl.log")
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0].run, records[1].run,
        "the new record names the held copy"
    );
    assert_ne!(records[0].mtime, records[1].mtime);
    fx.every_record_verifies();
}

/// **A TORN RECEIPT LOG IS ENDED BEFORE THE RECEIPT**: a hand-written line
/// cut short is not joined onto the new receipt, which stays the newest whole
/// line `api` reads.
#[test]
fn a_torn_receipt_log_is_ended_before_the_receipt() {
    let fx = Fixture::standard("torn-receipt");
    fs::write(
        fx.root.join(RECEIPTS),
        "copied=2026-10-01T00:00:00Z dest=/Vol",
    )
    .unwrap();
    fx.backup();
    let receipts = fx.receipts();
    assert_eq!(receipts.len(), 2, "{receipts:?}");
    assert_eq!(receipts[0], "copied=2026-10-01T00:00:00Z dest=/Vol");
    assert!(receipts[1].starts_with("copied=") && receipts[1].contains(" by=cli-backup "));
}

/// **A RECEIPT LOG THAT IS NOT A REGULAR FILE IS REFUSED BEFORE A COPY**: the
/// run could never record itself.
#[test]
fn a_receipt_log_that_is_a_link_is_refused_before_a_copy() {
    let fx = Fixture::standard("receipt-link");
    std::os::unix::fs::symlink(fx.base.join("elsewhere.log"), fx.root.join(RECEIPTS)).unwrap();
    let refused = fx
        .backup_with(false, Inject::default(), u64::MAX)
        .expect_err("refused");
    assert_eq!(refused.word, "receipt_unwritable");
    assert_eq!(fs::read_dir(&fx.target).unwrap().count(), 0);
}

/// **THE PLAN WRITES NOTHING ON EITHER SIDE**, and says what a copy would do.
#[test]
fn the_plan_writes_nothing_on_either_side() {
    let fx = Fixture::standard("plan");
    fs::create_dir_all(fx.root.join("logs")).unwrap();
    fs::create_dir_all(fx.root.join("results")).unwrap();
    let (code, out) = fx.verb(&["--to", &fx.to(), "--plan"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.starts_with("BACKUP PLAN -- to "), "{out}");
    assert!(
        out.contains("nothing was written") && out.contains(" new, 0 changed"),
        "{out}"
    );
    assert!(
        out.contains("NOT COVERED") && out.contains("logs, results"),
        "what the copy leaves out is named: {out}"
    );
    assert_eq!(fs::read_dir(&fx.target).unwrap().count(), 0);
    assert!(fx.receipts().is_empty());
    let planned = fx.backup_with(true, Inject::default(), u64::MAX).unwrap();
    assert_eq!(planned.run, 0);
    assert_eq!(planned.total().copied, 0);
    assert!(planned.total().copy_bytes > 0);
}

/// **A TAMPERED COPY FAILS THE DRILL BY NAME**: one byte of a copy flipped on
/// the target is a digest mismatch, the result is `fail`, and the verb exits
/// FAILED.
#[test]
fn a_tampered_copy_fails_the_drill_by_name() {
    let fx = Fixture::standard("tampered");
    fx.backup();
    let record = fx
        .records()
        .into_iter()
        .find(|r| r.class == MONTH && r.path.ends_with(b".bin"))
        .unwrap();
    let copy = fx.copy_of(&record);
    let mut bytes = fs::read(&copy).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x5a;
    fs::write(&copy, bytes).unwrap();
    let (code, out) = fx.drill(&["--from", &fx.to()]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains(": FAIL: ") && out.contains("is not the manifest's"),
        "{out}"
    );
    let line = fs::read_to_string(fx.root.join(DRILLS)).unwrap();
    assert_eq!(words_of(line.trim_end(), "result"), Some("fail"));
}

/// **MORE MONTHS ASKED THAN HELD IS INCOMPLETE, SAID**, never a pass; a
/// target with no manifest and a scratch folder inside the store are refused.
#[test]
fn a_drill_asked_for_more_months_than_held_is_incomplete() {
    let fx = Fixture::standard("incomplete");
    let (code, out) = fx.drill(&["--from", &fx.to()]);
    assert_eq!(code, FAILED);
    assert!(out.contains("BACKUP DRILL REFUSED (no_manifest)"), "{out}");
    fx.backup();
    let (code, out) = fx.drill(&["--from", &fx.to(), "--months", "4"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains(": INCOMPLETE: 3 of 4 month(s) restored (the manifest holds 3)"),
        "{out}"
    );
    let root = fx.root.clone();
    let store = move || Ok(root.clone());
    let mut out = String::new();
    let code = drill_command_with(&["--from", &fx.to()], &store, &fx.root, None, &mut out);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("BACKUP DRILL REFUSED (scratch_inside)"),
        "{out}"
    );
}

/// **OUT OF THE BOX: A TARGET INSIDE A SOURCE ARCHIVE IS REFUSED.** The tick
/// store root is an archive this verb copies; a copy inside it would copy
/// itself on every run.
#[test]
fn a_target_inside_a_source_archive_is_refused() {
    let fx = Fixture::standard("in-archive");
    let inside = fx.base.join("archives/ticks/backup");
    fs::create_dir_all(&inside).unwrap();
    // The target is on another device than the store: only the archive's
    // place refuses it.
    let devices = |path: &Path| -> io::Result<u64> { Ok(u64::from(path.starts_with(&inside))) };
    let free = |_: &Path| -> Result<u64, String> { Ok(u64::MAX) };
    let env = Env {
        commit: None,
        device: &devices,
        free: &free,
        inject: Inject::default(),
    };
    let ask = Ask {
        to: inside.clone(),
        plan: false,
        bytes_per_s: 0,
    };
    let refused = backup(&fx.root, &ask, &env).expect_err("refused");
    assert_eq!(refused.word, "inside_archive", "{refused:?}");
    assert_eq!(fs::read_dir(&inside).unwrap().count(), 0);
}

/// **OUT OF THE BOX: A TARGET WHOSE PATH HOLDS A SPACE AND A PERCENT SIGN.**
/// The receipt's `dest=` stays one word (percent-encoded), so `api` reads the
/// whole path back, and the copy works.
#[test]
fn a_target_path_with_a_space_and_a_percent_sign_is_one_receipt_word() {
    let fx = Fixture::standard("odd-name");
    let odd = fx.target.join("My Passport 100%");
    fs::create_dir_all(&odd).unwrap();
    let (code, out) = fx.verb(&["--to", &odd.display().to_string()]);
    assert_eq!(code, OK, "{out}");
    let receipt = fx.receipts().pop().unwrap();
    let dest = words_of(&receipt, "dest").unwrap();
    assert!(!dest.contains(' '), "{dest}");
    assert_eq!(backup_decoded_str(dest), Some(odd.display().to_string()));
    assert!(receipt.len() <= MAX_LINE);
}

// ------------------------------------------------------------ combinations

/// **COMBINATION, AND THE SPEC'S OUT-OF-THE-BOX CASE: A TARGET THAT FILLS UP
/// MID-COPY WHILE A MONTH IS WRITER-HELD.** The run stops at the full disk:
/// what it copied is whole and recorded, the held month is not copied, the
/// manifest says `stopped`, and no receipt claims a copy. With room and the
/// month released, the next run finishes everything and every record holds.
#[test]
fn a_target_that_fills_up_mid_copy_while_a_month_is_writer_held_then_resumes() {
    let fx = Fixture::standard("full-and-busy");
    let unit = fx.bin("CASH", "ZZSHARE", false, 2);
    let writer = BarFile::open_or_create(
        &fx.root,
        month_path("CASH", "ZZSHARE", false, 2),
        symbol_id("ZZSHARE"),
    )
    .unwrap();
    let stopped = fx
        .backup_with(
            false,
            Inject {
                space: Some(40_000),
                ..Inject::default()
            },
            u64::MAX,
        )
        .expect_err("the disk filled");
    assert_eq!(stopped.word, "target_failed");
    assert!(
        stopped.why.contains("No space left on device"),
        "{}",
        stopped.why
    );
    assert!(fx.manifest().contains("\nstopped "), "{}", fx.manifest());
    assert!(fx.receipts().is_empty(), "a stopped run appends no receipt");
    assert!(
        !fx.records()
            .iter()
            .any(|r| r.unit.as_deref() == Some(unit.as_bytes()))
    );
    fx.every_record_verifies();
    drop(writer);

    let resumed = fx.backup();
    assert!(resumed.complete(), "{resumed:?}");
    assert!(
        fx.records()
            .iter()
            .any(|r| r.unit.as_deref() == Some(unit.as_bytes()))
    );
    fx.every_record_verifies();
    assert_eq!(fx.receipts().len(), 1);
}

/// **COMBINATION: A KILL, THEN A CHANGED SOURCE, THEN THE RESUME.** The file
/// changed between the two runs is copied once, as the store now holds it.
#[test]
fn a_kill_then_a_changed_source_then_a_resume() {
    let fx = Fixture::standard("kill-change");
    let _ = fx
        .backup_with(
            false,
            Inject {
                kill_after: Some(0),
                ..Inject::default()
            },
            u64::MAX,
        )
        .expect_err("killed");
    let mut ledger = File::options()
        .append(true)
        .open(fx.root.join("audit/scrub-blocks.gdfl.log"))
        .unwrap();
    ledger.write_all(b"run 2 invented\n").unwrap();
    drop(ledger);
    let resumed = fx.backup();
    assert!(resumed.complete());
    let records: Vec<Record> = fx
        .records()
        .into_iter()
        .filter(|r| r.path == b"audit/scrub-blocks.gdfl.log")
        .collect();
    assert_eq!(records.len(), 1, "copied once, as it is now");
    assert_eq!(
        fs::read(fx.copy_of(&records[0])).unwrap(),
        b"run 1 invented\nrun 2 invented\n"
    );
    fx.every_record_verifies();
}

/// **COMBINATION: A DRILL WHILE ONE MONTH IS BUSY AND ANOTHER MOVED SINCE THE
/// COPY.** The busy month is skipped and named, the next month in the order is
/// restored in its place, and the moved month is compared with its digest
/// only (`moved`), not failed.
#[test]
fn a_drill_while_one_month_is_busy_and_another_moved() {
    let fx = Fixture::standard("busy-moved");
    // A FOURTH MONTH, so three can still be restored with one busy.
    let path = month_path("INDEX", "NIFTY", false, 4);
    let mut writer = BarFile::open_or_create(&fx.root, path, symbol_id("NIFTY")).unwrap();
    let batch = bars(4, 0, 3);
    writer.append(&batch).unwrap();
    drop(writer);
    let held = vec![CensusHeld::new(
        Entry {
            key: EntryKey {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Index,
                symbol: Symbol::new("NIFTY").unwrap(),
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2025, 4).unwrap(),
            },
            rows: 3,
            first_ts_micros: batch[0].ts_micros,
            last_ts_micros: batch[2].ts_micros,
        },
        Closes::UNKNOWN,
    )];
    census_store::record(&fx.root, Vendor::Gdfl, &held).unwrap();
    fx.backup();
    let busy = BarFile::open_or_create(
        &fx.root,
        month_path("CASH", "ZZSHARE", false, 2),
        symbol_id("ZZSHARE"),
    )
    .unwrap();
    let mut moved = BarFile::open_or_create(
        &fx.root,
        month_path("FNO", "NIFTY", true, 3),
        symbol_id("NIFTY"),
    )
    .unwrap();
    moved.append(&bars(3, 100, 2)).unwrap();
    drop(moved);
    let (code, out) = fx.drill(&["--from", &fx.to(), "--months", "3"]);
    drop(busy);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(&format!(
            "BUSY            {}",
            fx.bin("CASH", "ZZSHARE", false, 2)
        )),
        "{out}"
    );
    assert!(out.contains(": PASS: 3 of 3"), "{out}");
    let line = fs::read_to_string(fx.root.join(DRILLS)).unwrap();
    assert_ne!(words_of(line.trim_end(), "moved"), Some("0"), "{line}");
    assert_eq!(words_of(line.trim_end(), "busy"), Some("1"));
}

// ------------------------------------------------------------ O(1) and audit

/// **THE PER-FILE STEP PROBES THE MANIFEST ONCE.** The book is a map built
/// once per run from the manifest; each listed file costs exactly one probe,
/// on the first run and on a run over a fuller manifest alike, never a
/// rescan.
#[test]
fn the_per_file_step_probes_the_manifest_once() {
    let fx = Fixture::standard("probes");
    for _ in 0..3 {
        let ran = fx.backup();
        assert_eq!(ran.probes, ran.total().listed, "{ran:?}");
    }
}

/// **EVERY STATE CHANGE OF BOTH VERBS IS AN AUDITED EVENT** (D-4893): a
/// start, a finish and a refusal, at the levels they are emitted at.
#[test]
fn every_backup_state_change_is_audited() {
    use telemetry::Level;
    for (level, message) in [
        (Level::Info, "backup started"),
        (Level::Info, "backup finished"),
        (Level::Error, "backup finished"),
        (Level::Error, "backup refused"),
        (Level::Info, "backup drill started"),
        (Level::Info, "backup drill finished"),
        (Level::Error, "backup drill finished"),
        (Level::Error, "backup drill refused"),
    ] {
        assert!(
            telemetry::is_audited(level, TARGET, message),
            "{level:?} {TARGET} {message}"
        );
    }
}

/// **THE PER-FILE STEP, MEASURED** (`docs/06-limits.md`): one probe of the
/// book and one record formatted, n = 1,000 sequential calls each over books
/// of 10,000 and 100,000 paths. Run with `--ignored --nocapture`; the numbers
/// go to the report.
#[test]
#[ignore = "a measurement for the report, not a gate"]
fn measure_the_per_file_step() {
    let mut p99s = Vec::new();
    for size in [10_000_u64, 100_000] {
        let mut book = Book::default();
        for n in 0..size {
            let path = format!("bars/gdfl/NSE/FNO/NIFTY/X{n}/1min/2025-03.bin");
            book.insert(
                path.as_bytes(),
                Held {
                    len: n,
                    mtime: 1,
                    run: 1,
                    digest: [7; 32],
                    archive: false,
                },
            );
        }
        let mut times = Vec::with_capacity(1_000);
        for n in 0..1_000_u64 {
            let path = format!("bars/gdfl/NSE/FNO/NIFTY/X{}/1min/2025-03.bin", n * 7 % size);
            let started = Instant::now();
            let held = book.probe(path.as_bytes());
            let line = record_line(
                2,
                MONTH,
                n,
                1,
                &[7; 32],
                path.as_bytes(),
                Some(path.as_bytes()),
            );
            times.push(started.elapsed());
            assert!(held.is_some() && !line.is_empty());
        }
        times.sort();
        let (p50, p99, max) = (times[499], times[989], times[999]);
        println!("book={size} n=1000 p50={p50:?} p99={p99:?} max={max:?}");
        p99s.push(p99);
    }
    assert!(
        p99s[1] < p99s[0] * 20 + Duration::from_micros(50),
        "{p99s:?}"
    );
}

// ------------------------------------------------------------ the parts

#[test]
fn a_value_is_encoded_as_the_journal_encodes_it_and_decoded_back() {
    let raw = b"/Volumes/My Passport/100%\n\xff~._-";
    let text = encoded(raw);
    assert_eq!(text, "/Volumes/My%20Passport/100%25%0A%FF~._-");
    assert_eq!(decoded(&text).as_deref(), Some(&raw[..]));
    for broken in ["%", "%2", "%zz", "a b", "%+1", "é"] {
        assert_eq!(decoded(broken), None, "{broken:?}");
    }
    assert_eq!(unhex(&hex(&[0xab; 32])), Some([0xab; 32]));
    assert_eq!(unhex("ab"), None);
    assert_eq!(unhex(&"g".repeat(64)), None);
    assert_eq!(utc(1_000_000_000), "2001-09-09T01:46:40Z");
    assert_eq!(utc(0), "1970-01-01T00:00:00Z");
}

#[test]
fn a_record_is_read_strictly() {
    let line = record_line(9, MONTH, 5, -3, &[1; 32], b"bars/a b", Some(b"bars/a b"));
    let record = record_of(&line).unwrap();
    assert_eq!(
        (record.run, record.len, record.mtime, record.path.as_slice()),
        (9, 5, -3, &b"bars/a b"[..])
    );
    let plain = record_line(9, "ops", 5, 0, &[1; 32], b"ops/x", None);
    assert!(record_of(&plain).is_some());
    for bad in [
        format!("{line} extra=1"),
        line.replace("run=9", "run=x"),
        line.replace(" unit=", " unix="),
        plain.replace("class=ops", "class=month"),
        format!("{plain} unit=bars/x"),
        line.replace("path=bars/a%20b", "path="),
        "file".to_owned(),
        "end 1".to_owned(),
    ] {
        assert_eq!(record_of(&bad), None, "{bad}");
    }
}

#[test]
fn the_drill_order_spans_kinds_and_rotates() {
    let units: Vec<Vec<u8>> = [
        "bars/gdfl/NSE/INDEX/NIFTY/1min/2025-01.bin",
        "bars/gdfl/NSE/INDEX/NIFTY/1min/2025-02.bin",
        "bars/gdfl/NSE/CASH/ZZSHARE/1min/2025-02.bin",
        "bars/gdfl/NSE/FNO/NIFTY/X/1min/2025-03.bin",
    ]
    .iter()
    .map(|u| u.as_bytes().to_vec())
    .collect();
    let first = drill_order(&units, 0);
    assert_eq!(first.len(), 4, "every month once");
    let kinds: Vec<Vec<u8>> = first.iter().take(3).map(|u| kind_of(u)).collect();
    assert_eq!(
        kinds,
        [b"CASH".to_vec(), b"FNO".to_vec(), b"INDEX".to_vec()]
    );
    let next = drill_order(&units, 1);
    assert_ne!(first, next, "the next drill starts elsewhere in a kind");
    assert!(drill_order(&[], 3).is_empty());
}

/// The drill line keeps under the receipt cap however many months it names.
#[test]
fn a_drill_line_never_passes_the_line_cap() {
    let line = DrillLine {
        drilled_at: 1_000_000_000,
        from: PathBuf::from("/Volumes/B"),
        drill: 1,
        result: "pass".to_owned(),
        months: 500,
        asked: 500,
        held: 500,
        files: 1,
        bytes: 1,
        moved: 0,
        failed: 0,
        busy: 0,
        scratch: PathBuf::from("/tmp/x"),
        commit: None,
        units: (0..500)
            .map(|n| format!("bars/gdfl/NSE/INDEX/N{n}/1s/2025-01.bin"))
            .collect(),
    }
    .line();
    assert!(line.len() <= MAX_LINE, "{}", line.len());
    assert!(line.contains(" more="), "{line}");
    assert!(line.starts_with("drilled=2001-09-09T01:46:40Z from=/Volumes/B drill=1 result=pass"));
}

/// **A MANIFEST LINE THE DISK DID NOT FINISH IS ENDED BEFORE THE NEXT ONE**:
/// a full disk mid-append left a fragment, and the `stopped` line written
/// after it was joined onto it into one line no run could read.
#[test]
fn a_manifest_line_the_disk_did_not_finish_is_ended_before_the_next() {
    let fx = fixture("ledger-torn");
    let path = fx.target.join(MANIFEST);
    let mut ledger = Ledger::open(&path, false).unwrap();
    ledger
        .line("run 5 started=1 commit=unstamped store=/x")
        .unwrap();
    ledger.file.write_all(b"file run=5 class=ops len=").unwrap();
    ledger.torn = true;
    ledger.line("stopped 5 why=full").unwrap();
    ledger.sync().unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.ends_with(&format!(
            "file run=5 class=ops len={VOID}\nstopped 5 why=full\n"
        )),
        "{text}"
    );
    let book = Book::load(&path, false).expect("every line reads");
    assert_eq!(book.newest_run, 5);
    assert_eq!(book.files, 0);
}

/// **THE ONE-TIME PASS'S RATE, SAMPLED** (`docs/06-limits.md`): one run over
/// a store holding the three months and a 256 MiB invented archive, its
/// wall time and bytes per second. Run with `--ignored --nocapture`; the
/// rate goes to the report as a sample, and a full copy's time is an
/// extrapolation from it.
#[test]
#[ignore = "a measurement for the report, not a gate"]
fn measure_the_copy_rate() {
    let fx = Fixture::standard("rate");
    let big = fx.base.join("archives/big.zip");
    let chunk: Vec<u8> = (0..CHUNK).map(|n| u8::try_from(n % 251).unwrap()).collect();
    let mut file = File::create(&big).unwrap();
    for _ in 0..256 {
        file.write_all(&chunk).unwrap();
    }
    file.sync_all().unwrap();
    drop(file);
    fx.journal(&[&big]);
    let started = Instant::now();
    let ran = fx.backup();
    let took = started.elapsed();
    let bytes = ran.total().copied_bytes;
    let files = ran.total().copied;
    let rate = u128::from(bytes) * 1_000 / took.as_millis().max(1) / (1024 * 1024);
    println!("copied {files} files {bytes} bytes in {took:?}: {rate} MiB/s");
    assert!(bytes >= 256 * CHUNK as u64);
    fx.every_record_verifies();
}
