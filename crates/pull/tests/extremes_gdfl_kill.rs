//! A capital-market GDFL import killed at seven points of its run and
//! resumed, for the cloud extremes round of 10 Nov 2026
//! (`EXTREMES-REPORT.md`). Its own test binary, because the child it spawns
//! is this binary re-run on this one test.
//!
//! `gdfl_import_tests.rs`'s kill test drives the OPTIONS tree. This drives
//! the stocks tree (one-second bars of a share, the journal, the census and
//! the day ledger) from an archive on disk: the run is killed by `SIGKILL`
//! at each eighth of a clean run's duration; after each kill every month the
//! store holds opens through the reader door at a whole committed prefix of
//! the clean month, and a resume over the same archive lands every bar file
//! byte-identical to an uninterrupted import.
//!
//! Every value is invented at run time.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::gdfl_archive::Archive;
use pull::gdfl_import::{ImportKind, Run, run_cm};
use pull::session::Day;
use store::file::{BarFile, StoreError};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The variable that turns this binary into the importing child.
const CHILD: &str = "BRUTEX_EXTREMES_GDFL_KILL_CHILD";

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The sessions imported: eight trading days of July 2024.
fn days() -> Vec<Day> {
    [1_u8, 2, 3, 4, 5, 8, 9, 10]
        .iter()
        .map(|&d| day(2024, 7, d))
        .collect()
}

fn le16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A plain zip of `(name, bytes, deflate)` members.
fn zip_of(members: &[(String, Vec<u8>, bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data, deflate) in members {
        let packed = if *deflate {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data).unwrap();
            enc.finish().unwrap()
        } else {
            data.clone()
        };
        let method = if *deflate { 8 } else { 0 };
        let crc = pull::gdfl_archive::crc32(data);
        let offset = u32::try_from(out.len()).unwrap();
        let sizes = [
            u32::try_from(packed.len()).unwrap(),
            u32::try_from(data.len()).unwrap(),
        ];
        let name_len = u16::try_from(name.len()).unwrap();
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, method);
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, sizes[0]);
        le32(&mut out, sizes[1]);
        le16(&mut out, name_len);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0);
        le16(&mut central, method);
        le32(&mut central, 0);
        le32(&mut central, crc);
        le32(&mut central, sizes[0]);
        le32(&mut central, sizes[1]);
        le16(&mut central, name_len);
        for _ in 0..4 {
            le16(&mut central, 0);
        }
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = u32::try_from(out.len()).unwrap();
    let count = u16::try_from(members.len()).unwrap();
    let size = u32::try_from(central.len()).unwrap();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, count);
    le16(&mut out, count);
    le32(&mut out, size);
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

/// A RELIANCE day file: a trade every second of the session, its price
/// walking, its quantity varying.
fn reliance(d: Day) -> Vec<u8> {
    let mut text = format!("{}\r\n", pull::gdfl_cm::HEADER_OPEN_INTEREST);
    let date = format!("{:02}/{:02}/{:04}", d.day(), d.month(), d.year());
    for sod in 33_300_u32..55_800 {
        let paisa = 290_000 + i64::from(sod % 701) * 5 + i64::from(d.day());
        let _infallible = std::fmt::Write::write_fmt(
            &mut text,
            format_args!(
                "RELIANCE.NSE,{date},{:02}:{:02}:{:02},{}.{:02},0,0,0,0,{},0\r\n",
                sod / 3_600,
                sod / 60 % 60,
                sod % 60,
                paisa / 100,
                paisa % 100,
                1 + sod % 9
            ),
        );
    }
    text.into_bytes()
}

/// Writes the outer archive of every day to `path`.
fn write_archive(path: &Path) {
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let outer: Vec<(String, Vec<u8>, bool)> = days()
        .into_iter()
        .map(|d| {
            let folder = format!(
                "GFDLCM_STOCK_TICK_{:02}{:02}{:04}",
                d.day(),
                d.month(),
                d.year()
            );
            let inner = zip_of(&[(format!("{folder}/RELIANCE.NSE.csv"), reliance(d), true)]);
            let month = MONTHS[usize::from(d.month()) - 1];
            let name = format!("STOCKS/{}/{month}_{}/{folder}.zip", d.year(), d.year());
            (name, inner, false)
        })
        .collect();
    std::fs::write(path, zip_of(&outer)).unwrap();
}

/// One import of every day from the archive at `archive` into `store`.
fn import(archive: &Path, store: &Path) -> pull::gdfl_import::Report {
    let days = days();
    let only = vec!["RELIANCE".to_owned()];
    let run = Run {
        kind: ImportKind::Stocks,
        from: days[0],
        to: days[days.len() - 1],
        only: &only,
        store_root: store,
    };
    run_cm(&Archive::open(archive).unwrap(), &run).unwrap()
}

/// The one-second RELIANCE bars of July 2024, or none.
fn july(store: &Path) -> Result<Vec<store::format::Bar>, StoreError> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol: "RELIANCE",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 7).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    match BarFile::open_existing(
        store,
        path,
        pull::ingest::repair_plan::symbol_id("RELIANCE"),
    ) {
        Ok(file) => (0..file.records()).map(|i| file.read_record(i)).collect(),
        Err(StoreError::Missing { .. }) => Ok(Vec::new()),
        Err(other) => Err(other),
    }
}

/// Every `.bin` and `.crc` under `store/bars`, by relative path.
fn bar_files(store: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![store.join("bars")];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "bin" || e == "crc") {
                let rel = path.strip_prefix(store).unwrap().to_path_buf();
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

#[test]
fn a_stocks_import_killed_at_seven_points_resumes_to_the_clean_store() {
    if let Some(dirs) = std::env::var_os(CHILD) {
        let dirs = PathBuf::from(dirs);
        let _report = import(&dirs.join("cm.zip"), &dirs.join("S"));
        return;
    }
    let root = std::env::temp_dir().join(format!("brutex-xgk-{}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let archive = root.join("cm.zip");
    write_archive(&archive);
    let clean_store = root.join("Clean");
    let started = std::time::Instant::now();
    let clean = import(&archive, &clean_store);
    let took = started.elapsed();
    assert!(clean.failures.is_empty(), "{:?}", clean.failures);
    assert_eq!(clean.days_imported, 8);
    let want = bar_files(&clean_store);
    let whole = july(&clean_store).unwrap();
    assert_eq!(whole.len(), 8 * 22_500);

    let mut failures = Vec::new();
    let mut interrupted = 0;
    for eighth in 1..=7_u32 {
        let run = root.join(format!("Kill{eighth}"));
        std::fs::create_dir_all(&run).unwrap();
        std::fs::hard_link(&archive, run.join("cm.zip")).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "a_stocks_import_killed_at_seven_points_resumes_to_the_clean_store",
                "--test-threads=1",
            ])
            .env(CHILD, &run)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(took * eighth / 8);
        child.kill().unwrap();
        let status = child.wait().unwrap();
        let store = run.join("S");
        match july(&store) {
            Ok(held) => {
                if held[..] != whole[..held.len()] {
                    failures.push(format!(
                        "kill {eighth}: {} bars are not a prefix of the clean month",
                        held.len()
                    ));
                }
                if !status.success() && held.len() < whole.len() {
                    interrupted += 1;
                }
            }
            Err(why) => failures.push(format!("kill {eighth}: the month refused: {why}")),
        }
        let resumed = import(&run.join("cm.zip"), &store);
        if !resumed.failures.is_empty() {
            failures.push(format!(
                "kill {eighth}: the resume failed: {:?}",
                resumed.failures
            ));
        }
        if bar_files(&store) != want {
            failures.push(format!(
                "kill {eighth}: the resumed store differs from the clean one"
            ));
        }
    }
    let _gone = std::fs::remove_dir_all(&root);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        interrupted >= 2,
        "only {interrupted} of 7 kills landed inside the run"
    );
}
