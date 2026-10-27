#![cfg(test)]
//! BASE-COV-PULL (D-7331): the lines of `gdfl_import` the crate's tests did
//! not reach on 27 Oct 2026 (Route B, `state/fix/round2/base-cov-pull-gaps-a1.txt`):
//! the origin measure's caps and faults, the rescan's refusals, the journal
//! reader's skipped lines and the per-day branches of the rescan. Every value
//! is invented at run time; no vendor row is quoted.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::Path;

use super::*;
use crate::gdfl_fixtures::{Method, put, scratch, zip};
use crate::gdfl_nfo::NfoTickStore;
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The offset of the end-of-central-directory record of a fixture zip with
/// no comment: its last 22 bytes.
fn end_of(zip: &[u8]) -> usize {
    zip.len() - 22
}

/// A central directory stated one byte over the cap is refused by name and
/// never read; one stated exactly at the cap is not "over the cap" (it is
/// then past the end of this small file, which is its own refusal).
#[test]
fn a_central_directory_over_the_cap_is_refused_by_name() {
    let root = scratch("cov-cd-cap");
    let one = zip(&[("a.csv", b"one", Method::Stored)]);
    let over_cap = u32::try_from(CENTRAL_DIRECTORY_CAP + 1).unwrap();
    let mut over = one.clone();
    let end = end_of(&over);
    over[end + 12..end + 16].copy_from_slice(&over_cap.to_le_bytes());
    let path = put(&root, "over.zip", &over);
    let why = central_directory_bytes(&path).unwrap_err();
    assert!(
        why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
        "{why}"
    );
    let at_cap = u32::try_from(CENTRAL_DIRECTORY_CAP).unwrap();
    let mut level = one;
    level[end + 12..end + 16].copy_from_slice(&at_cap.to_le_bytes());
    let path = put(&root, "level.zip", &level);
    let why = central_directory_bytes(&path).unwrap_err();
    assert!(
        why.contains("lies past the end") && !why.contains("over the cap"),
        "{why}"
    );
    // Through the origin: a single outer zip over the cap is measured for its
    // bytes and named, its digest `unmeasured`.
    let origin = Origin::measure(SourceKind::Zip, &root.join("over.zip"), None);
    assert_eq!(origin.archive_bytes, u64::try_from(over.len()).unwrap());
    assert_eq!(origin.digest_hex(), "unmeasured");
    assert!(
        origin
            .fault
            .as_deref()
            .is_some_and(|why| why.contains("over the cap")),
        "{origin:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A single outer "zip" that is no zip is measured for its bytes and its
/// fault named (the outer-file arm of `measure_zips`), and an empty file is
/// one too.
#[test]
fn a_single_file_that_is_no_zip_is_measured_and_named() {
    let root = scratch("cov-not-zip");
    let path = put(&root, "plain.zip", b"not a zip at all");
    let origin = Origin::measure(SourceKind::Zip, &path, Some("c0ffee"));
    assert_eq!(origin.archive_bytes, 16);
    assert_eq!(origin.digest, None);
    assert!(
        origin
            .fault
            .as_deref()
            .is_some_and(|why| why.contains("plain.zip") && why.contains("no end-of-central")),
        "{origin:?}"
    );
    let empty = put(&root, "empty.zip", b"");
    let origin = Origin::measure(SourceKind::Zip, &empty, None);
    assert_eq!((origin.archive_bytes, origin.digest), (0, None));
    assert!(origin.fault.is_some(), "{origin:?}");
    std::fs::remove_dir_all(root).unwrap();
}

/// Out of the box: a path that is neither a file nor a directory (a
/// character device) is not listed as a directory of yearly zips; the
/// listing's refusal is named, nothing is counted.
#[test]
fn a_zip_source_that_is_a_device_is_named_not_walked() {
    let origin = Origin::measure(SourceKind::Zip, Path::new("/dev/null"), None);
    assert_eq!((origin.archive_bytes, origin.digest), (0, None));
    assert!(
        origin
            .fault
            .as_deref()
            .is_some_and(|why| why.starts_with("/dev/null: ")),
        "{origin:?}"
    );
}

/// A yearly zip that is a dangling link: its size cannot be read, the first
/// fault named is its own, the year beside it still counts its bytes, and
/// no digest is claimed for a set with a hole.
#[test]
fn a_yearly_zip_that_is_a_dangling_link_is_the_named_fault() {
    let root = scratch("cov-dangling-year");
    let a = zip(&[("a.csv", b"one", Method::Stored)]);
    put(&root, "years/2018.zip", &a);
    std::os::unix::fs::symlink(root.join("nowhere.zip"), root.join("years/2019.zip")).unwrap();
    let origin = Origin::measure(SourceKind::Zip, &root.join("years"), None);
    assert_eq!(origin.archive_bytes, u64::try_from(a.len()).unwrap());
    assert_eq!(origin.digest, None);
    let fault = origin.fault.unwrap();
    assert!(fault.contains("2019.zip"), "{fault}");
    assert!(!fault.contains("2018.zip"), "{fault}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The tick store's walk stops past its cap and says so, by the root's name
/// and the cap; at exactly the cap it does not stop (the boundary), and with
/// a cap of zero the first entry stops it. A walk that stopped claims no
/// digest even though `FORMAT.md` reads (the fault is kept, not replaced).
#[test]
fn the_tick_store_walk_stops_past_its_cap_and_says_so() {
    let root = scratch("cov-walk-cap");
    let ts = root.join("ts");
    put(&ts, "FORMAT.md", b"# an invented format");
    put(&ts, "cm/a.bts", b"12");
    // Entries: FORMAT.md and cm under the root, a.bts under cm: three.
    let level = measure_tick_store_within(&ts, 3);
    assert_eq!((level.bytes, level.fault.as_deref()), (2, None));
    assert!(level.digest.is_some());
    let over = measure_tick_store_within(&ts, 2);
    let fault = over.fault.as_deref().unwrap();
    assert!(
        fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
        "{fault}"
    );
    assert_eq!(over.digest, None, "a stopped walk claims no digest");
    let zero = measure_tick_store_within(&ts, 0);
    assert!(zero.fault.is_some_and(|why| why.contains("more than 0 entries")));
    assert_eq!(zero.bytes, 0);
    // Production passes the documented cap.
    assert_eq!(measure_tick_store(&ts).fault, None);
    std::fs::remove_dir_all(root).unwrap();
}

/// A `FORMAT.md` one byte over its cap is refused by size, never read; one
/// exactly at the cap is hashed (the boundary).
#[test]
fn a_format_file_over_its_cap_is_refused_by_size() {
    use sha2::Digest as _;
    let root = scratch("cov-format-cap");
    let cap = usize::try_from(FORMAT_CAP).unwrap();
    let ts = root.join("ts");
    put(&ts, "FORMAT.md", &vec![b'#'; cap + 1]);
    put(&ts, "x/y.bts", b"1234");
    let over = Origin::measure(SourceKind::TickStore, &ts, None);
    assert_eq!((over.archive_bytes, over.digest), (4, None));
    let fault = over.fault.unwrap();
    assert!(
        fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
        "{fault}"
    );
    let level = vec![b'#'; cap];
    put(&ts, "FORMAT.md", &level);
    let at = Origin::measure(SourceKind::TickStore, &ts, None);
    let want: [u8; 32] = sha2::Sha256::digest(&level).into();
    assert_eq!((at.digest, at.fault), (Some(want), None));
    std::fs::remove_dir_all(root).unwrap();
}

/// The three hole reasons a repair reads, by their codes, and nothing else:
/// the two holes no repair fills, another code, an empty word and a code in
/// another case are not repairable (D-6900 keep item 2).
#[test]
fn only_the_three_repairable_holes_are_repairable() {
    for reason in ["needs_month_generation", "month_superseded", "repair_refused"] {
        assert!(hole_is_repairable(reason), "{reason}");
    }
    for reason in [
        "bars_differ_held",
        "repair_impossible",
        "month_busy",
        "",
        "REPAIR_REFUSED",
        "repair_refused ",
    ] {
        assert!(!hole_is_repairable(reason), "{reason:?}");
    }
}

/// `options` read from a capital-market source is refused by name before
/// anything is read, and a backwards range is refused before the journal is
/// looked at, under every kind.
#[test]
fn a_rescan_refuses_options_from_a_cm_source_and_a_backwards_range() {
    let root = scratch("cov-rescan-refusals");
    let source = TickStore::new(&root.join("ts"));
    let range = (day(2019, 5, 30), day(2019, 5, 30));
    assert_eq!(
        rescan_cm(&source, ImportKind::Options, range, &root.join("S")).unwrap_err(),
        ImportRefusal::KindUnknown {
            word: "options read from a capital-market source".to_owned()
        }
    );
    let (from, to) = (day(2019, 5, 31), day(2019, 5, 30));
    // The journal path is a directory: had the journal been read first, this
    // would be a journal refusal.
    std::fs::create_dir_all(journal_path(&root.join("S"))).unwrap();
    for kind in [ImportKind::Indices, ImportKind::Stocks] {
        assert_eq!(
            rescan_cm(&source, kind, (from, to), &root.join("S")).unwrap_err(),
            ImportRefusal::RangeBackwards { from, to }
        );
    }
    let nfo = NfoTickStore::new(&root.join("ts"));
    assert_eq!(
        rescan_nfo(&nfo, (from, to), &root.join("S"), false).unwrap_err(),
        ImportRefusal::RangeBackwards { from, to }
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The journal reader: a missing journal is no imported day; a journal that
/// is a directory is refused by name; a line too short to key, a line of
/// another kind, a day that is no day and a verb that closes nothing are
/// passed over; a key's last closing line decides, `refused` included.
#[test]
fn the_journal_reader_keys_every_line_it_can_and_passes_over_the_rest() {
    let root = scratch("cov-journal-reader");
    let store = root.join("S");
    assert!(imported_days(&store, ImportKind::Indices).unwrap().is_empty());
    std::fs::create_dir_all(journal_path(&store)).unwrap();
    let refused = imported_days(&store, ImportKind::Indices).unwrap_err();
    assert!(
        matches!(&refused, ImportRefusal::Journal { why } if why.contains("gdfl.journal")),
        "{refused:?}"
    );
    std::fs::remove_dir(journal_path(&store)).unwrap();
    let text = [
        "done indices 2019-05-30 * files=1",
        "done indices",
        "done",
        "",
        "done stocks 2019-05-29 *",
        "done indices 2019-13-01 *",
        "done indices yesterday *",
        "incomplete indices 2019-05-28 *",
        "refused indices 2019-05-28 *",
        "refused indices 2019-05-27 *",
        "recounted indices 2019-05-27 *",
        "run indices 2019-05-24 *",
        "started indices 2019-05-23 *",
        "refused indices 2019-05-22 NIFTY",
        "done indices 2019-05-22 *",
    ]
    .join("\n");
    put(&store, "imports/gdfl.journal", text.as_bytes());
    let mut got: Vec<Day> = imported_days(&store, ImportKind::Indices)
        .unwrap()
        .into_iter()
        .collect();
    got.sort_unstable();
    assert_eq!(
        got,
        [day(2019, 5, 22), day(2019, 5, 27), day(2019, 5, 30)],
        "the 28th's last line refused it; the 29th is stocks"
    );
    let stocks: Vec<Day> = imported_days(&store, ImportKind::Stocks)
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(stocks, [day(2019, 5, 29)]);
    assert!(imported_days(&store, ImportKind::Options).unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// A rescan of options without the lineage reads no lineage ledger and
/// writes none: a range the journal never imported reads nothing.
#[test]
fn an_options_rescan_without_lineage_reads_nothing_it_was_not_asked() {
    let root = scratch("cov-rescan-nfo");
    let store = root.join("S");
    let nfo = NfoTickStore::new(&root.join("ts"));
    let done = rescan_nfo(&nfo, (day(2019, 5, 27), day(2019, 5, 31)), &store, false).unwrap();
    assert_eq!(
        (done.days_read, done.days_not_imported, done.lineage_lines),
        (0, 5, 0),
        "{done:?}"
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert!(
        !store.join("imports").exists(),
        "nothing was written for a range never imported"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A tick at `sod`.
fn tick(sod: u32) -> Tick {
    Tick {
        sod,
        ltp: 12_345,
        ltq: 1,
        oi: None,
    }
}

/// One handed file of `ticks`, named `name`.
fn handed(name: &str, ticks: Vec<Tick>) -> Handed {
    Handed {
        file: TickFile {
            symbol: "INVENTEDX".to_owned(),
            contract: None,
            name: name.to_owned(),
            ticks,
        },
        facts: FileFacts::default(),
    }
}

/// The rescan's per-day branches, with a reader that answers what each day
/// asks: a day the journal imported that the calendar declines (a Sunday) is
/// a calendar failure and is not read; a day the source does not hold is
/// missing, its refusals and its refused file named; a held day with a file
/// stamped past the day counts that file refused. Several at once in one
/// range (a combination of extremes).
#[test]
fn a_rescan_names_a_declined_day_a_missing_day_and_a_refused_file() {
    let root = scratch("cov-rescan-days");
    let store = root.join("S");
    let (sunday, missing, held) = (day(2019, 6, 2), day(2019, 5, 30), day(2019, 5, 31));
    let journal = format!(
        "done indices {sunday} *\ndone indices {missing} *\ndone indices {held} *\n"
    );
    put(&store, "imports/gdfl.journal", journal.as_bytes());
    let mut asked: Vec<Day> = Vec::new();
    let done = rescan_core(
        ImportKind::Indices,
        (missing, sunday),
        &store,
        |on, _run, sink| {
            asked.push(on);
            if on == missing {
                sink(handed("past.csv", vec![tick(86_400)]));
                let read = DayRead {
                    held: false,
                    skipped: 0,
                    refused: vec![Refused::at(
                        Cause::open(Stage::File, "invented_refusal"),
                        Failure::new("member.csv", "an invented refusal"),
                    )],
                };
                return (read, DayExtras::default());
            }
            sink(handed("late.csv", vec![tick(86_399)]));
            (
                DayRead {
                    held: true,
                    ..DayRead::default()
                },
                DayExtras::default(),
            )
        },
    )
    .unwrap();
    assert_eq!(asked, [missing, held], "the Sunday is never read");
    assert_eq!(done.days_missing, [missing]);
    assert_eq!(done.days_read, 1);
    assert_eq!(done.days_not_imported, 1, "June 1st, a Saturday, was never imported");
    assert_eq!((done.files, done.files_refused), (1, 2), "{done:?}");
    let named: Vec<(String, String)> = done
        .failures
        .iter()
        .map(|f| (f.instrument.clone(), f.why.clone()))
        .collect();
    assert!(
        named
            .iter()
            .any(|(what, _)| what == &format!("{sunday} calendar")),
        "{named:?}"
    );
    assert!(
        named
            .iter()
            .any(|(what, why)| what == &format!("{missing} past.csv") && why.contains("86400")),
        "{named:?}"
    );
    assert!(
        named
            .iter()
            .any(|(what, why)| what == &format!("{missing} member.csv")
                && why == "an invented refusal"),
        "{named:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
