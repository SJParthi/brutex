//! IA-17b: the `.auc` of a repaired CASH month (D-4875 (5), D-4898; spec §8
//! items 2 and 3). A reader resolves the generation every reader opens; an
//! append never writes inside a served revision; a day the sidecar can take
//! only with a new generation is carried, never refused and never written;
//! and the gathered repair's step writes the new generation's whole sidecar,
//! repeatably, only inside the revision it is handed.
//!
//! Every value is invented at run time and no vendor row is quoted: the
//! prices are chosen so they cannot be mistaken for real ones.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};

use super::*;
use crate::gdfl_fixtures::scratch;

/// The store's id the month files are written under.
const SYMBOL_ID: u32 = 7;

fn day(d: u8) -> Day {
    Day::new(2026, 9, d).unwrap()
}

fn key() -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, "SYNTHREV").unwrap()
}

/// An invented print at 15:28:07: prices no share of this name ever had.
fn print(price: i64, quantity: u64) -> Print {
    Print {
        sod: 15 * 3_600 + 28 * 60 + 7,
        price,
        quantity,
        rows: 1,
        line: 9,
    }
}

/// The share's one-second month of September 2026.
fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol: "SYNTHREV",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2026, 9).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// One invented bar at 10:00 of `on`.
fn bar(on: Day, price: i64) -> store::format::Bar {
    let secs = i64::from(on.days_from_epoch()) * 86_400 + 10 * 3_600 - IST_OFFSET_SECS;
    store::format::Bar {
        ts_micros: secs * 1_000_000,
        open: price,
        high: price + 5,
        low: price - 5,
        close: price + 1,
        volume: 3,
        open_interest: store::format::OI_NULL,
    }
}

/// A store whose month holds `on`'s bar in its original.
fn original_holding(tag: &str, on: Day) -> PathBuf {
    let store = scratch(tag);
    let mut file = store::file::BarFile::open_or_create(&store, month(), SYMBOL_ID).unwrap();
    file.append(&[bar(on, 712_340)]).unwrap();
    store
}

/// The month repaired with `on`'s bar through the gathered repair's step,
/// `carried` handed to [`write_revision`] before the promotion: served from
/// revision 1 from here on. The revision's root and what the step wrote.
fn repaired_with(store: &Path, on: Day, carried: &[(Day, &[Print])]) -> (PathBuf, Option<usize>) {
    let policy = store::repair::Policy {
        open_interest_applies: false,
        basis_witness: false,
    };
    let mut written = None;
    let mut step = |revision: &Path| -> Result<(), String> {
        written = write_revision(store, revision, &key(), day(1), carried)?;
        Ok(())
    };
    let done = store::repair::repair_month_with(
        store,
        month(),
        SYMBOL_ID,
        &[bar(on, 712_010)],
        policy,
        Some(&mut step),
    )
    .unwrap();
    assert!(matches!(done, store::repair::MonthRepair::Repaired { .. }));
    (
        store::repair::revision_root(store, store::repair::Revision::new(1).unwrap()),
        written,
    )
}

/// Revision 1 of the month, holding `on`'s bar beside the original's,
/// PUBLISHED AND NOT PROMOTED: where the step runs (D-4898).
fn published(store: &Path, on: Day) -> PathBuf {
    let original = store::file::BarFile::open_existing(store, month(), SYMBOL_ID).unwrap();
    let header = original.header();
    let mut merged: Vec<store::format::Bar> = (0..header.n_valid)
        .map(|at| original.read_record(at).unwrap())
        .collect();
    drop(original);
    merged.push(bar(on, 712_010));
    merged.sort_by_key(|bar| bar.ts_micros);
    let revision = store::repair::Revision::new(1).unwrap();
    store::repair::publish(store, month(), SYMBOL_ID, revision, header, &merged).unwrap();
    assert_eq!(
        store::generation::served(store, month()).unwrap().ordinal(),
        0,
        "published, not served"
    );
    store::repair::revision_root(store, revision)
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    out
}

/// The IST days of a sidecar's records, in file order.
fn days_of(sidecar: &Sidecar) -> Vec<i64> {
    sidecar
        .records
        .iter()
        .map(|record| ist_day(record.t_micros))
        .collect()
}

/// **A DAY OLDER THAN THE SIDECAR HOLDS IS CARRIED, NEVER REFUSED.** The
/// original holds the 24th; the 22nd's prints are answered
/// `WithGeneration` with the sidecar's own words, and the file is the byte
/// it was. The 24th offered again is held (`written` 0); no prints is
/// nothing. [`append`] itself still refuses the older day by name.
#[test]
fn a_day_older_than_the_sidecar_holds_is_carried_never_refused() {
    let store = original_holding("ia17b-auc-older", day(24));
    let held = [print(712_365, 40)];
    append(&store, &key(), day(24), &held).unwrap();
    let path = path(&store, &key(), day(24)).unwrap();
    let bytes = std::fs::read(&path).unwrap();

    let carried = file_or_carry(&store, &key(), day(22), &[print(711_965, 11)]).unwrap();
    let DayPrints::WithGeneration(why) = carried else {
        panic!("{carried:?}");
    };
    assert!(why.contains("holds a day after 2026-09-22"), "{why}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "nothing written");
    assert_eq!(
        file_or_carry(&store, &key(), day(24), &held).unwrap(),
        DayPrints::Appended(Appended {
            offered: 1,
            written: 0
        })
    );
    assert_eq!(
        file_or_carry(&store, &key(), day(22), &[]).unwrap(),
        DayPrints::Appended(Appended {
            offered: 0,
            written: 0
        })
    );
    assert!(append(&store, &key(), day(22), &[print(711_965, 11)]).is_err());
    std::fs::remove_dir_all(store).unwrap();
}

/// **A MONTH SERVED FROM A REVISION (spec §8 item 3, the hazard).** Its
/// readers read the revision's sidecar ([`path`]); an [`append`] into it is
/// refused by name and writes no byte anywhere, neither in the revision nor
/// in the original; [`file_or_carry`] answers a day the served sidecar holds
/// `written` 0, carries a day it lacks, and refuses a day it holds
/// differently.
#[test]
fn a_revision_served_month_is_never_appended_to_and_its_readers_read_the_served_sidecar() {
    let store = original_holding("ia17b-auc-revision", day(24));
    let held = [print(712_365, 40)];
    append(&store, &key(), day(24), &held).unwrap();
    let carried = [(day(22), &[print(711_965, 11)][..])];
    let (revision, written) = repaired_with(&store, day(22), &carried);
    assert_eq!(written, Some(2), "the 24th served, the 22nd carried");
    assert_eq!(
        store::generation::served(&store, month())
            .unwrap()
            .ordinal(),
        1
    );
    assert_eq!(
        path(&store, &key(), day(24)).unwrap(),
        month().to_path_buf(&revision).with_extension(EXTENSION)
    );
    let before = tree(&store);

    let refused = append(&store, &key(), day(25), &[print(712_765, 9)]).unwrap_err();
    assert!(
        refused.to_string().contains("served from revision 1"),
        "{refused}"
    );
    assert_eq!(tree(&store), before, "no byte anywhere");
    assert_eq!(
        file_or_carry(&store, &key(), day(24), &held).unwrap(),
        DayPrints::Appended(Appended {
            offered: 1,
            written: 0
        })
    );
    let DayPrints::WithGeneration(why) =
        file_or_carry(&store, &key(), day(25), &[print(712_765, 9)]).unwrap()
    else {
        panic!("the 25th is carried");
    };
    assert!(why.contains("served from revision 1"), "{why}");
    let differs = file_or_carry(&store, &key(), day(24), &[print(712_399, 40)]).unwrap_err();
    assert!(differs.to_string().contains("different"), "{differs}");
    assert_eq!(tree(&store), before, "no byte anywhere");
    std::fs::remove_dir_all(store).unwrap();
}

/// **THE STEP WRITES THE WHOLE SIDECAR, IN DAY ORDER, IN VERSION 2.** The
/// served sidecar is a VERSION-1 file holding the 24th; the carried 22nd
/// and 23rd join it, and the revision's file is byte for byte the version-2
/// sidecar an in-order import of the three days writes. The served file is
/// the byte it was.
#[test]
fn the_step_merges_the_served_sidecar_and_the_carried_days_in_day_order() {
    let store = original_holding("ia17b-auc-merge", day(24));
    let original = path(&store, &key(), day(24)).unwrap();
    let mut v1 = header_of(VERSION_1).to_vec();
    v1.extend_from_slice(&Record::of(day(24), &print(712_365, 40)).encode(VERSION_1, 0));
    std::fs::write(&original, &v1).unwrap();
    let revision = published(&store, day(22));
    let (p22, p23) = ([print(711_965, 11)], [print(712_165, 22)]);

    let written = write_revision(
        &store,
        &revision,
        &key(),
        day(1),
        &[(day(23), &p23[..]), (day(22), &p22[..])],
    )
    .unwrap();

    assert_eq!(written, Some(3));
    let target = month().to_path_buf(&revision).with_extension(EXTENSION);
    let sidecar = read(&target).unwrap().unwrap();
    assert_eq!(sidecar.version, VERSION);
    let utc = |d: u8| i64::from(day(d).days_from_epoch());
    assert_eq!(days_of(&sidecar), vec![utc(22), utc(23), utc(24)]);
    let in_order = scratch("ia17b-auc-in-order");
    for (d, prints) in [
        (22, &p22[..]),
        (23, &p23[..]),
        (24, &[print(712_365, 40)][..]),
    ] {
        append(&in_order, &key(), day(d), prints).unwrap();
    }
    assert_eq!(
        std::fs::read(&target).unwrap(),
        std::fs::read(path(&in_order, &key(), day(22)).unwrap()).unwrap()
    );
    assert_eq!(
        std::fs::read(&original).unwrap(),
        v1,
        "the served file is read only"
    );
    std::fs::remove_dir_all(store).unwrap();
    std::fs::remove_dir_all(in_order).unwrap();
}

/// **A CARRIED DAY THE SERVED SIDECAR HOLDS DIFFERENTLY IS REFUSED.** One
/// day is never served with two readings: the step refuses, naming the
/// day, and writes no file into the revision (the repair then stops before
/// the promotion). The same day carried alike is kept once.
#[test]
fn the_step_refuses_a_carried_day_the_served_sidecar_holds_differently() {
    let store = original_holding("ia17b-auc-differs", day(24));
    append(&store, &key(), day(24), &[print(712_365, 40)]).unwrap();
    let revision = published(&store, day(22));
    let target = month().to_path_buf(&revision).with_extension(EXTENSION);

    let refused = write_revision(
        &store,
        &revision,
        &key(),
        day(1),
        &[(day(24), &[print(712_366, 40)][..])],
    )
    .unwrap_err();

    assert!(refused.contains("2026-09-24 holds 1 records"), "{refused}");
    assert!(!target.exists(), "nothing written into the revision");
    assert_eq!(
        write_revision(
            &store,
            &revision,
            &key(),
            day(1),
            &[(day(24), &[print(712_365, 40)][..])]
        )
        .unwrap(),
        Some(1)
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// **REPEATABLE, AND NOTHING FOR NOTHING.** No served sidecar and nothing
/// carried writes no file (a month before the closing auction's era). A
/// write the disk cuts short leaves no target, and the retry writes it
/// whole; a target a crash tore is replaced whole; a target already holding
/// exactly these bytes is left as it is.
#[test]
fn the_step_is_repeatable_and_writes_nothing_for_nothing() {
    let store = original_holding("ia17b-auc-repeat", day(24));
    let revision = published(&store, day(22));
    let target = month().to_path_buf(&revision).with_extension(EXTENSION);
    assert_eq!(
        write_revision(&store, &revision, &key(), day(1), &[]).unwrap(),
        None
    );
    assert!(!target.exists());
    let no_prints: [(Day, &[Print]); 1] = [(day(23), &[])];
    assert_eq!(
        write_revision(&store, &revision, &key(), day(1), &no_prints).unwrap(),
        None,
        "a carried day with no print carries nothing"
    );
    assert!(!target.exists());

    let carried = [(day(22), &[print(711_965, 11)][..])];
    seam::FAIL_AFTER.with(|after| after.set(Some(20)));
    let full = write_revision(&store, &revision, &key(), day(1), &carried).unwrap_err();
    assert!(full.contains("no space left"), "{full}");
    assert!(!target.exists(), "a cut write is never the target");
    assert_eq!(
        write_revision(&store, &revision, &key(), day(1), &carried).unwrap(),
        Some(1)
    );
    let whole = std::fs::read(&target).unwrap();
    std::fs::write(&target, &whole[..HEADER_LEN + 5]).unwrap();
    assert_eq!(
        write_revision(&store, &revision, &key(), day(1), &carried).unwrap(),
        Some(1)
    );
    assert_eq!(std::fs::read(&target).unwrap(), whole, "replaced whole");
    let modified = std::fs::metadata(&target).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert_eq!(
        write_revision(&store, &revision, &key(), day(1), &carried).unwrap(),
        Some(1)
    );
    assert_eq!(
        std::fs::metadata(&target).unwrap().modified().unwrap(),
        modified,
        "the same bytes are not written again"
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// **A MONTH WHOSE SERVED GENERATION CANNOT BE TOLD TAKES NO PRINT**
/// (D-TBD-IA-17b-1 (2)): a generation pointer that is not a regular file
/// (here a directory at its name) refuses the append by name, never
/// guessing that the original is served. (A pointer shorter than one record
/// is a torn tail the store reads past, `store::generation::served`.)
#[test]
fn a_month_whose_served_generation_cannot_be_told_takes_no_print() {
    let store = original_holding("ia17b-auc-pointer", day(24));
    let pointer = store::generation::pointer_path(&store, month());
    std::fs::create_dir_all(&pointer).unwrap();
    let before = tree(&store);

    let refused = append(&store, &key(), day(22), &[print(711_965, 11)]).unwrap_err();

    assert!(
        refused
            .to_string()
            .contains("served generation cannot be told"),
        "{refused}"
    );
    assert_eq!(tree(&store), before, "nothing written");
    std::fs::remove_dir_all(store).unwrap();
}

/// **A DAY'S RECORDS ARE THOSE OF ITS IST DATE, MIDNIGHT TO MIDNIGHT**
/// (IA-AUC-06, D-TBD-IA-17b-1 (3); clock edges): a record on the day's
/// first IST microsecond is the day's, one a microsecond before it is the
/// day before's, and one on the next IST midnight is the next day's, so a
/// generation's carried prints are matched to the day they were filed for
/// and to no other, whatever the hour of the print.
#[test]
fn a_days_records_are_those_of_its_ist_date_midnight_to_midnight() {
    let on = day(23);
    let midnight = (i64::from(on.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    let at = |t_micros: i64| Record {
        t_micros,
        price_paisa: 123_457,
        quantity: 7,
        rows: 1,
    };
    let records = [
        at(midnight - 1),
        at(midnight),
        at(midnight + 86_400 * 1_000_000 - 1),
        at(midnight + 86_400 * 1_000_000),
    ];

    let got: Vec<i64> = records_of_day(&records, on)
        .iter()
        .map(|record| record.t_micros)
        .collect();

    assert_eq!(
        got,
        [midnight, midnight + 86_400 * 1_000_000 - 1],
        "the day's first and last IST microsecond, and nothing of its neighbours"
    );
}
