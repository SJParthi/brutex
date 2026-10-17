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
    // THE MOVE IS RECORDED, SO IT IS NOT REDONE (AG-02, rekey-3): no month
    // is read or written again.
    assert!(
        again.contains("APPLIED: 0 key(s) moved (0 month file(s) written, 0 already held exactly)"),
        "{again}"
    );
    assert!(
        again.contains("PENDING: 0 (0 to move, 0 to retire, 0 unresolved, 0 census)"),
        "{again}"
    );
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
    assert!(
        narrowed.contains(
            "ledger              0 superseded and 0 retired key(s) already recorded (BANKNIFTY)"
        ),
        "{narrowed}"
    );
    assert!(narrowed.contains("PENDING: 0"), "{narrowed}");
}

/// rekey-3: AFTER AN APPLY, `--plan` PRINTS WHAT IS RECORDED APART FROM WHAT
/// IS PENDING: the move and the retirement listed as recorded, nothing to
/// move or retire, `PENDING: 0`, and the "would change nothing" suffix that
/// a recorded move used to suppress for ever. Before the apply the same
/// receipt reads the move and the retirement as pending. The ledger totals
/// follow `--underlying`.
#[test]
fn the_plan_receipt_prints_recorded_apart_from_pending() {
    let scratch = store("recorded");
    let root = scratch.0.clone();
    let mut before = String::new();
    assert_eq!(
        command_with(&["--plan"], &|| Ok(root.clone()), &mut before),
        OK,
        "{before}"
    );
    assert!(
        before.contains(
            "1 to move, 1 to retire, 0 unresolved; 0 moved and 0 retired already recorded"
        ),
        "{before}"
    );
    assert!(
        before.contains("PENDING: 2 (1 to move, 1 to retire, 0 unresolved, 0 census)"),
        "{before}"
    );
    let mut out = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut out),
        OK,
        "{out}"
    );

    let mut after = String::new();
    assert_eq!(
        command_with(
            &["--plan", "--underlying", "NIFTY"],
            &|| Ok(root.clone()),
            &mut after
        ),
        OK,
        "{after}"
    );
    assert!(
        after.contains(
            "0 to move, 0 to retire, 0 unresolved; 1 moved and 1 retired already recorded"
        ),
        "{after}"
    );
    assert!(
        after.contains("PENDING: 0 (0 to move, 0 to retire, 0 unresolved, 0 census)"),
        "{after}"
    );
    assert!(
        after.contains(&format!(
            "moved   NSE-FNO-NIFTY-{STATED} -> NSE-FNO-NIFTY-{SETTLED} (recorded in the ledger; 1 month(s), 1 bar(s))"
        )),
        "{after}"
    );
    assert!(
        after.contains(&format!(
            "retired NSE-FNO-NIFTY-{IMPOSSIBLE} (recorded in the ledger)"
        )),
        "{after}"
    );
    assert!(
        after.contains(
            "ledger              1 superseded and 1 retired key(s) already recorded (NIFTY)"
        ),
        "{after}"
    );
    assert!(after.contains("(and would change nothing)"), "{after}");
    assert!(
        after.contains("census: lists no key the ledger withdraws (step 4 done) (counted over the whole ledger and census; --underlying does not narrow it)"),
        "{after}"
    );
    assert!(!after.contains("    move    "), "{after}");
    let mut whole = String::new();
    let _ = command_with(&["--plan"], &|| Ok(root.clone()), &mut whole);
    assert!(
        whole.contains("1 superseded and 1 retired key(s) already recorded (the whole ledger)"),
        "{whole}"
    );
    assert!(
        !whole.contains("--underlying does not narrow it"),
        "{whole}"
    );
}

/// AG-02 repair r1e, blocking item 1: AN APPLY STOPPED AFTER ITS LEDGER
/// LINES AND BEFORE THE CENSUS EPOCH (killed, or refused by a census lock
/// another ingest holds) leaves the retirement recorded and the census still
/// listing it. The receipt names that step, counts it in `PENDING`, and does
/// not say "would change nothing"; the next `--apply` installs the epoch;
/// after it `PENDING: 0` means what the acceptance says: a second `--apply`
/// writes no month, appends no line and installs no epoch. The base receipt
/// read `PENDING: 0` and "would change nothing" in the retirements-only
/// shape of this state.
#[test]
fn the_plan_receipt_names_a_census_step_left_pending() {
    let scratch = store("census-step");
    let root = scratch.0.clone();
    let found = pull::gdfl_rekey::survey(&root, None).unwrap();
    let decided = pull::gdfl_rekey::plan(&found, &pull::gdfl_rekey::Withdrawn::default());
    let ledger = pull::gdfl_lineage::lineage_path(&root);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(&ledger, format!("{}\n", decided.retired[0].line())).unwrap();

    let mut stopped = String::new();
    assert_eq!(
        command_with(&["--plan"], &|| Ok(root.clone()), &mut stopped),
        OK,
        "{stopped}"
    );
    assert!(
        stopped.contains("census: 1 withdrawn key(s) still listed"),
        "{stopped}"
    );
    assert!(
        stopped.contains("PENDING: 2 (1 to move, 0 to retire, 0 unresolved, 1 census)"),
        "{stopped}"
    );
    assert!(
        stopped.contains(&format!(
            "    census  NSE-FNO-NIFTY-{IMPOSSIBLE} (withdrawn by the ledger, still listed)"
        )),
        "{stopped}"
    );
    assert!(!stopped.contains("would change nothing"), "{stopped}");

    let mut finish = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut finish),
        OK,
        "{finish}"
    );
    assert!(
        finish.contains("census epoch 000001 -> 000002 (rekey)"),
        "{finish}"
    );

    let mut done = String::new();
    assert_eq!(
        command_with(&["--plan"], &|| Ok(root.clone()), &mut done),
        OK,
        "{done}"
    );
    assert!(
        done.contains("census: lists no key the ledger withdraws (step 4 done)"),
        "{done}"
    );
    assert!(
        done.contains("PENDING: 0 (0 to move, 0 to retire, 0 unresolved, 0 census)"),
        "{done}"
    );
    assert!(done.contains("(and would change nothing)"), "{done}");
    let bytes = std::fs::read(&ledger).unwrap();
    let mut again = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut again),
        OK,
        "{again}"
    );
    assert!(
        again.contains("APPLIED: 0 key(s) moved (0 month file(s) written, 0 already held exactly)"),
        "{again}"
    );
    assert!(again.contains("0 ledger line(s) appended"), "{again}");
    assert!(
        again.contains("census lists no withdrawn key; no epoch was installed"),
        "{again}"
    );
    assert_eq!(std::fs::read(&ledger).unwrap(), bytes);
}

/// A census that will not read leaves step 4 unknown: the receipt says why,
/// counts it in `PENDING`, and the plan exits failed rather than clean.
#[test]
fn a_census_that_will_not_read_is_named_and_fails_the_plan() {
    let scratch = store("census-garbage");
    let root = scratch.0.clone();
    let found = pull::gdfl_rekey::survey(&root, None).unwrap();
    let decided = pull::gdfl_rekey::plan(&found, &pull::gdfl_rekey::Withdrawn::default());
    let ledger = pull::gdfl_lineage::lineage_path(&root);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(&ledger, format!("{}\n", decided.retired[0].line())).unwrap();
    std::fs::write(
        epoch_path(&root, Vendor::Gdfl, Segment::Fno, 2),
        b"not a census",
    )
    .unwrap();
    let mut out = String::new();
    assert_eq!(
        command_with(&["--plan"], &|| Ok(root.clone()), &mut out),
        FAILED,
        "{out}"
    );
    assert!(out.contains("census: could not be read ("), "{out}");
    assert!(out.contains("step 4 is not known done"), "{out}");
    assert!(out.contains("unresolved, 1 census)"), "{out}");
    assert!(!out.contains("would change nothing"), "{out}");
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

/// AG-02 repair r2: `--apply` while another apply holds the re-key's lock
/// stops by the lock's name, writes no month and no ledger line, and fails;
/// once the lock is free the same command applies.
#[test]
fn an_apply_while_the_rekey_lock_is_held_stops_and_writes_nothing() {
    let scratch = store("lockheld");
    let root = scratch.0.clone();
    let at = pull::gdfl_rekey::ledger_lock_path(&root);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    let file = std::fs::File::create(&at).unwrap();
    let held = store::flock::Flock::try_lock(&file, at.as_path()).unwrap();
    let mut out = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut out),
        FAILED,
        "{out}"
    );
    assert!(
        out.contains("GDFL RE-KEY STOPPED:")
            && out
                .contains("gdfl.lineage.lock is held by another re-key apply; nothing was written"),
        "{out}"
    );
    assert!(!pull::gdfl_lineage::lineage_path(&root).exists(), "{out}");
    held.release().unwrap();
    let mut again = String::new();
    assert_eq!(
        command_with(&["--apply"], &|| Ok(root.clone()), &mut again),
        OK,
        "{again}"
    );
    assert!(pull::gdfl_lineage::lineage_path(&root).exists(), "{again}");
}
