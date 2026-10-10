//! `cli gdfl-rekey` driven end to end on a scratch store of invented bars
//! (D-3331, D-3316; WP-12c). No vendor row is copied: every key, price and
//! count is made up.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path};
use pull::session::Day;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{USAGE, command_with};
use crate::{FAILED, MISUSED, OK};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-rekey-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).unwrap();
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const STATED: &str = "2023-06-29-1000000-PE";
const SETTLED: &str = "2023-06-28-1000000-PE";
const IMPOSSIBLE: &str = "2020-06-25-191250000-CE";

fn parts<'a>(symbol: &'a str, contract: &str, month: YearMonth) -> PathParts<'a> {
    PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol,
        contract: Some(Contract::parse(contract).unwrap()),
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the store's symbol id: the low half of the FNV-1a"
)]
fn id() -> u32 {
    brutex_core::universe::fnv1a("NIFTY") as u32
}

/// Writes one invented bar per `(day, second of day)` into `contract`'s
/// months, and returns the census rows of those months.
fn put(root: &Path, contract: &str, at: &[(Day, u32)]) -> Vec<Held> {
    let mut rows = Vec::new();
    for &(day, sod) in at {
        let month = day.year_month().unwrap();
        let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + i64::from(sod)) * 1_000_000;
        let bar = Bar {
            ts_micros: ts,
            open: 41,
            high: 41,
            low: 41,
            close: 41,
            volume: 3,
            open_interest: 30,
        };
        let path = StorePath::new(parts("NIFTY", contract, month)).unwrap();
        let mut file = BarFile::open_or_create(root, path, id()).unwrap();
        file.append(&[bar]).unwrap();
        let header = file.header();
        rows.push(Held::new(
            Entry {
                key: EntryKey {
                    contract: Contract::parse(contract),
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: Segment::Fno,
                    symbol: Symbol::new("NIFTY").unwrap(),
                    timeframe: Timeframe::SECOND_1,
                    month,
                },
                rows: header.n_valid,
                first_ts_micros: header.first_ts_micros,
                last_ts_micros: header.last_ts_micros,
            },
            Closes::known(41, 41).unwrap(),
        ));
    }
    rows
}

/// A migrated store holding a stated-2023-06-29 put, an impossible strike
/// and the ladder that makes it impossible, every key counted.
fn store(tag: &str) -> Scratch {
    let scratch = Scratch::new(tag);
    let root = &scratch.0;
    let on = Day::new(2019, 5, 30).unwrap();
    let mut rows = Vec::new();
    for (at, key) in [
        "2019-05-30-1150000-CE",
        "2019-05-30-1170000-PE",
        "2019-05-30-1180000-CE",
    ]
    .into_iter()
    .enumerate()
    {
        rows.extend(put(root, key, &[(on, 33_300 + u32::try_from(at).unwrap())]));
    }
    rows.extend(put(root, IMPOSSIBLE, &[(on, 40_000)]));
    rows.extend(put(
        root,
        STATED,
        &[(Day::new(2020, 6, 15).unwrap(), 34_000)],
    ));
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64).unwrap();
    for row in rows {
        census.record_held(row).unwrap();
    }
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, census.image()).unwrap();
    scratch
}

/// Arguments it does not know are a misuse, refused before anything is read.
#[test]
fn an_unknown_or_repeated_argument_is_a_misuse_and_reads_nothing() {
    let never = || -> Result<PathBuf, String> { panic!("the store was asked for") };
    for bad in [
        &[][..],
        &["--force"][..],
        &["--apply", "--apply"][..],
        &["--plan", "--apply"][..],
        &["--apply", "--plan"][..],
        &["--underlying", "NIFTY"][..],
        &["--plan", "--underlying"][..],
        &["--plan", "--underlying", "nifty"][..],
        &[
            "--plan",
            "--underlying",
            "NIFTY",
            "--underlying",
            "BANKNIFTY",
        ][..],
    ] {
        let mut out = String::new();
        assert_eq!(command_with(bad, &never, &mut out), MISUSED, "{bad:?}");
        assert!(out.contains(USAGE), "{out}");
    }
}

/// `--plan` names the move and the retirement and writes nothing at all.
#[test]
fn a_dry_run_names_the_plan_and_writes_nothing() {
    let scratch = store("dry");
    let root = scratch.0.clone();
    let mut out = String::new();
    let code = command_with(&["--plan"], &|| Ok(root.clone()), &mut out);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(&format!("move    NSE-FNO-NIFTY-{STATED} -> NSE-FNO-NIFTY-{SETTLED} (vendor_redated; 1 month(s), 1 bar(s))")),
        "{out}"
    );
    assert!(
        out.contains(&format!(
            "retire  contract_refused: NSE-FNO-NIFTY-{IMPOSSIBLE}"
        )),
        "{out}"
    );
    assert!(out.contains("PLAN ONLY: nothing was written"), "{out}");
    assert!(!root.join("bars/gdfl/NSE/FNO/NIFTY").join(SETTLED).exists());
    assert!(!pull::gdfl_lineage::lineage_path(&root).exists());
}

/// `--apply` carries the plan out and exits OK; a second `--apply` changes
/// nothing and says so. `--underlying` naming another underlying plans
/// nothing.
#[test]
fn applying_moves_and_retires_and_a_second_apply_writes_nothing() {
    let scratch = store("apply");
    let root = scratch.0.clone();
    let mut out = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut out),
        OK,
        "{out}"
    );
    assert!(
        out.contains("APPLIED: 1 key(s) moved (1 month file(s) written"),
        "{out}"
    );
    assert!(
        out.contains("census epoch 000001 -> 000002 (rekey)"),
        "{out}"
    );
    assert!(
        root.join("bars/gdfl/NSE/FNO/NIFTY")
            .join(SETTLED)
            .join("1s/2020-06.bin")
            .exists()
    );
    let ledger = std::fs::read(pull::gdfl_lineage::lineage_path(&root)).unwrap();

    let mut again = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut again),
        OK,
        "{again}"
    );
    assert!(again.contains("1 already held exactly"), "{again}");
    assert!(again.contains("0 ledger line(s) appended"), "{again}");
    assert!(again.contains("no epoch was installed"), "{again}");
    assert_eq!(
        std::fs::read(pull::gdfl_lineage::lineage_path(&root)).unwrap(),
        ledger
    );

    let mut narrowed = String::new();
    let code = command_with(
        &["--plan", "--underlying", "BANKNIFTY"],
        &|| Ok(root.clone()),
        &mut narrowed,
    );
    assert_eq!(code, OK, "{narrowed}");
    assert!(narrowed.contains("PLAN: 0 key(s) judged"), "{narrowed}");
}

/// A store root that cannot be had refuses, and nothing is written.
#[test]
fn a_store_that_cannot_be_read_refuses() {
    let mut out = String::new();
    let code = command_with(&["--plan"], &|| Err("no store".to_owned()), &mut out);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("GDFL RE-KEY REFUSED: no store; nothing was written"),
        "{out}"
    );
}
