//! `cli census-recount` and `cli census-migrate`, driven against scratch
//! stores built from invented bars (D-3316, D-3317 item 5). No vendor row.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::ingest::census_store;
use pull::manifest::{
    Closes, Entry, EntryKey, FNO_FLOOR, Held, Manifest, epoch_path, manifest_path, vendor_dir,
};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{day, migrate_with, recount_arguments, recount_with};
use crate::{FAILED, MISUSED, OK};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-census-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(&root).expect("scratch root");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

/// 10:00 IST on day `day` of `month`, plus `n` seconds, in microseconds.
fn at(month: YearMonth, day: i64, n: i64) -> i64 {
    let (start, _) = month.ist_bounds_micros();
    start + ((day - 1) * 86_400 + 10 * 3_600 + n) * 1_000_000
}

/// `count` invented one-second option bars on `day` of `month`, closes
/// `700 + n` paisa — a price no listed contract of this strike ever quoted.
fn bars(month: YearMonth, day: i64, from: i64, count: i64) -> Vec<Bar> {
    (from..from + count)
        .map(|n| Bar {
            ts_micros: at(month, day, n),
            open: 700 + n,
            high: 710 + n,
            low: 690 + n,
            close: 700 + n,
            volume: 1,
            open_interest: 5_000,
        })
        .collect()
}

/// The invented contract `2019-12-26-1200000-CE`, as the store spells it.
fn contract() -> Contract {
    Contract::parse("2019-12-26-1200000-CE").expect("a contract")
}

/// Appends `batch` to one GDFL option month, through the store's writer.
fn write_option(root: &Path, month: YearMonth, batch: &[Bar]) {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: Some(contract()),
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a store path");
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file = store::file::BarFile::open_or_create(root, path, id).expect("open");
    let _ = file.append(batch).expect("append");
}

/// The census key of that month.
fn key(month: YearMonth) -> EntryKey {
    EntryKey {
        contract: Some(contract()),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month,
    }
}

/// A census row for that month claiming `rows` rows over the given span.
fn row(month: YearMonth, rows: u64, first: i64, last: i64) -> Held {
    Held::new(
        Entry {
            key: key(month),
            rows,
            first_ts_micros: first,
            last_ts_micros: last,
        },
        Closes::UNKNOWN,
    )
}

fn single(root: &Path) -> Manifest {
    let bytes = std::fs::read(manifest_path(root, Vendor::Gdfl)).expect("the census file");
    Manifest::open_image(Vendor::Gdfl, &bytes).expect("a single file")
}

fn partition(root: &Path, segment: Segment) -> (u32, Manifest) {
    let epoch = census_store::current_epoch(root, Vendor::Gdfl, segment)
        .expect("listed")
        .expect("an epoch");
    let census = census_store::read_epoch(
        &epoch_path(root, Vendor::Gdfl, segment, epoch),
        Vendor::Gdfl,
        segment,
    )
    .expect("a partition");
    (epoch, census)
}

/// An invented import journal: three census-refused days, one of them outside
/// a recount of 2019-11-14 onward, and a crashed day.
const JOURNAL: &str = "\
incomplete options 2019-11-06 * definition=2 files=1 seconds=6 failures=1
begin options 2019-11-28 *
incomplete options 2019-11-28 * definition=2 files=1 seconds=6 failures=1
begin options 2019-12-26 *
incomplete options 2019-12-26 * definition=2 files=1 seconds=9 failures=1
begin options 2019-12-27 *
";

fn journal(root: &Path) -> PathBuf {
    root.join("imports").join("gdfl.journal")
}

/// The arguments parse in the grammar the usage gives, and every other shape
/// is refused by name.
#[test]
fn the_recount_arguments_are_a_vendor_a_first_day_and_an_optional_last() {
    let (vendor, span) = recount_arguments(&["gdfl", "--from", "2019-11-14"]).expect("parses");
    assert_eq!(vendor, Vendor::Gdfl);
    assert_eq!(span.from, day("2019-11-14").expect("a day"));
    assert_eq!(span.to, None, "no --to is open-ended");
    let (_, span) =
        recount_arguments(&["gdfl", "--to", "2020-09-15", "--from", "2019-11-14"]).expect("parses");
    assert_eq!(span.to, Some(day("2020-09-15").expect("a day")));
    for (bad, says) in [
        (&["groww2"][..], "not a feed"),
        (&["gdfl"][..], "needs --from DAY"),
        (&["gdfl", "--from"][..], "needs a day"),
        (&["gdfl", "--from", "2019-11"][..], "not a YYYY-MM-DD day"),
        (&["gdfl", "--from", "2019-02-30"][..], "2019-02-30"),
        (
            &["gdfl", "--since", "2019-11-14"][..],
            "not a census-recount flag",
        ),
        (
            &["gdfl", "--from", "2019-11-14", "--from", "2019-12-01"][..],
            "given twice",
        ),
        (
            &["gdfl", "--from", "2020-01-01", "--to", "2019-12-31"][..],
            "is before",
        ),
    ] {
        let why = recount_arguments(bad).expect_err("refused");
        assert!(why.contains(says), "{bad:?}: {why}");
    }
    assert!(day("2019-11-1").is_err(), "a day is two digits");
}

/// THE CENSUS THE IMPORT COULD NOT PUBLISH IS COUNTED FROM THE FILES, AND ITS
/// DAYS ARE CLOSED (DB-G09, DB-G49, DB-G78; D-3317 item 5).
///
/// The real store's shape, in miniature: the census row for a contract's
/// November stops at the 13th with 6 rows while the file holds 12, and its
/// December has a file and no row at all. A recount from 2019-11-14 reads only
/// the two headers, records both rows exactly as the files state them, and
/// closes the two census-refused days it covers — not the one before it. A
/// second recount changes nothing, byte for byte.
#[test]
fn a_recount_records_what_the_bar_headers_hold_and_closes_its_days() {
    let scratch = Scratch::new("recount");
    let root = scratch.0.clone();
    let (nov, dec) = (ym(2019, 11), ym(2019, 12));
    write_option(&root, nov, &bars(nov, 13, 0, 6));
    // The census as the import left it: November cut at the 13th.
    let first = at(nov, 13, 0);
    assert_eq!(
        pull::ingest::record_held(&root, Vendor::Gdfl, &[row(nov, 6, first, at(nov, 13, 5))]),
        None
    );
    // Then the import kept writing bars the census never counted.
    write_option(&root, nov, &bars(nov, 28, 0, 6));
    write_option(&root, dec, &bars(dec, 26, 0, 9));
    std::fs::create_dir_all(journal(&root).parent().expect("imports")).expect("imports");
    std::fs::write(journal(&root), JOURNAL).expect("a journal");

    let store = || Ok(root.clone());
    let mut out = String::new();
    let code = recount_with(
        &["gdfl", "--from", "2019-11-14"],
        &store,
        Some("abc123"),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    for says in [
        "no vendor file was read",
        "in range            2",
        "recorded 2, already exact 0, refused 0",
        "days recounted      2 (options 2019-11-28 * .. options 2019-12-26 *)",
        "days left open      0",
    ] {
        assert!(out.contains(says), "{says:?} missing from {out}");
    }

    let census = single(&root);
    let nov_row = census.held(&key(nov)).expect("november");
    assert_eq!(
        nov_row.entry.rows, 12,
        "the file's count, not the census's 6"
    );
    assert_eq!(nov_row.entry.first_ts_micros, first);
    assert_eq!(nov_row.entry.last_ts_micros, at(nov, 28, 5));
    assert_eq!(
        nov_row.closes.paisa(),
        Some((700, 705)),
        "the file's own closes"
    );
    let dec_row = census.held(&key(dec)).expect("december now has a row");
    assert_eq!(dec_row.entry.rows, 9);
    assert_eq!(census.total_rows(), 21);

    let text = std::fs::read_to_string(journal(&root)).expect("the journal");
    let tail = text
        .strip_prefix(JOURNAL)
        .expect("appended, never rewritten");
    let lines: Vec<&str> = tail.lines().collect();
    assert_eq!(lines.len(), 3, "{tail}");
    assert!(lines[0].starts_with("run "), "{tail}");
    assert!(
        lines[0].contains(" source=recount commit=abc123 started="),
        "{tail}"
    );
    assert!(lines[0].ends_with(" from=2019-11-14 to=open"), "{tail}");
    let run = lines[0].split(' ').nth(1).expect("a run id");
    assert_eq!(
        lines[1],
        format!("recounted options 2019-11-28 * definition=2 months=1 run={run}")
    );
    assert_eq!(
        lines[2],
        format!("recounted options 2019-12-26 * definition=2 months=1 run={run}")
    );

    let before = std::fs::read(manifest_path(&root, Vendor::Gdfl)).expect("bytes");
    let mut again = String::new();
    assert_eq!(
        recount_with(&["gdfl", "--from", "2019-11-14"], &store, None, &mut again),
        OK,
        "{again}"
    );
    assert!(
        again.contains("recorded 0, already exact 2, refused 0"),
        "{again}"
    );
    assert!(again.contains("days recounted      0"), "{again}");
    assert_eq!(
        std::fs::read(manifest_path(&root, Vendor::Gdfl)).expect("bytes"),
        before,
        "an idempotent recount leaves the census byte for byte"
    );
    assert_eq!(
        std::fs::read_to_string(journal(&root)).expect("journal"),
        text
    );

    // A span that excludes both months offers nothing and writes nothing.
    let mut none = String::new();
    assert_eq!(
        recount_with(&["gdfl", "--from", "2020-01-01"], &store, None, &mut none),
        OK,
        "{none}"
    );
    assert!(none.contains("in range            0"), "{none}");
}

/// A DAY THE CENSUS CANNOT BE SHOWN TO HOLD IS LEFT OPEN, AND THE VERB FAILS:
/// a census-refused day no instrument-month spans, although it had begun a
/// write. A day whose files outnumber the months spanning it still closes,
/// and the receipt says so.
#[test]
fn a_day_the_recount_cannot_prove_is_left_open_and_the_verb_fails() {
    let scratch = Scratch::new("open");
    let root = scratch.0.clone();
    let nov = ym(2019, 11);
    write_option(&root, nov, &bars(nov, 28, 0, 3));
    std::fs::create_dir_all(journal(&root).parent().expect("imports")).expect("imports");
    let lines = "incomplete options 2019-11-27 * definition=2 files=1 seconds=3 failures=1\n\
                 incomplete options 2019-11-28 * definition=2 files=7 seconds=3 failures=1\n";
    std::fs::write(journal(&root), lines).expect("a journal");
    let store = || Ok(root.clone());
    let mut out = String::new();
    assert_eq!(
        recount_with(
            &["gdfl", "--from", "2019-11-27", "--to", "2019-11-28"],
            &store,
            None,
            &mut out
        ),
        FAILED,
        "{out}"
    );
    assert!(out.contains("days recounted      1"), "{out}");
    assert!(out.contains("files beyond months 1 closed day(s)"), "{out}");
    assert!(out.contains("days left open      1"), "{out}");
    assert!(
        out.contains(
            "options 2019-11-27 *: no FNO instrument-month the census holds spans the day"
        ),
        "{out}"
    );
    let text = std::fs::read_to_string(journal(&root)).expect("journal");
    let tail = text.strip_prefix(lines).expect("appended, never rewritten");
    assert!(
        tail.lines()
            .any(|line| line.starts_with("recounted options 2019-11-28 * definition=2 months=1 ")),
        "{tail}"
    );
    assert!(
        !tail.contains("options 2019-11-27"),
        "the open day gets no line: {tail}"
    );
}

/// A single-file census as the import left it: three writes of one option key
/// (two superseded) and one index month. Returns the file's bytes.
fn three_writes_and_an_index_month(root: &Path) -> Vec<u8> {
    let nov = ym(2019, 11);
    for (rows, last) in [(2, 1), (4, 3), (6, 5)] {
        assert_eq!(
            pull::ingest::record_held(
                root,
                Vendor::Gdfl,
                &[row(nov, rows, at(nov, 13, 0), at(nov, 13, last))]
            ),
            None
        );
    }
    let index = Held::new(
        Entry {
            key: EntryKey {
                contract: None,
                segment: Segment::Index,
                ..key(nov)
            },
            rows: 3,
            first_ts_micros: at(nov, 1, 0),
            last_ts_micros: at(nov, 1, 2),
        },
        Closes::known(1, 2).expect("prices"),
    );
    assert_eq!(
        pull::ingest::record_held(root, Vendor::Gdfl, &[index]),
        None
    );
    std::fs::read(manifest_path(root, Vendor::Gdfl)).expect("the single file")
}

/// A MIGRATION SPLITS BY SEGMENT, COMPACTS, AND LEAVES THE SINGLE FILE'S
/// BYTES ALONE (D-3316) — kept under the superseded name, its own name sealed
/// against readers built before version 4 (DB-B68).
#[test]
fn a_migration_compacts_into_epochs_and_leaves_the_single_file_untouched() {
    let scratch = Scratch::new("migrate");
    let root = scratch.0.clone();
    let before = three_writes_and_an_index_month(&root);
    let legacy = manifest_path(&root, Vendor::Gdfl);

    let store = || Ok(root.clone());
    let mut out = String::new();
    assert_eq!(migrate_with(&["gdfl"], &store, &mut out), OK, "{out}");
    assert!(out.contains("its bytes left exactly as they were"), "{out}");
    assert!(out.contains("4 entries, 2 keys, 9 rows"), "{out}");
    assert!(out.contains("000001.man"), "{out}");
    // SEALED AS IT IS SUPERSEDED (DB-B68): the bytes kept under the
    // superseded name, the name a stub every pre-version-4 reader refuses.
    let superseded = census_store::superseded_path(&root, Vendor::Gdfl);
    assert_eq!(std::fs::read(&superseded).expect("kept"), before);
    assert_eq!(
        std::fs::read(&legacy).expect("the stub"),
        census_store::sealed_stub(Vendor::Gdfl)
    );
    assert!(
        out.contains(
            "now refuses every reader built before version 4 (unknown manifest version 65535)"
        ),
        "{out}"
    );
    assert!(out.contains(census_store::V4_READER_SINCE), "{out}");

    let (epoch, fno) = partition(&root, Segment::Fno);
    assert_eq!(
        (epoch, fno.entries(), fno.keys(), fno.total_rows()),
        (1, 1, 1, 6)
    );
    assert_eq!(fno.capacity(), FNO_FLOOR);
    let (_, idx) = partition(&root, Segment::Index);
    assert_eq!((idx.entries(), idx.keys(), idx.total_rows()), (1, 1, 3));
    let (_, cash) = partition(&root, Segment::Cash);
    assert_eq!(cash.keys(), 0, "an empty segment is an empty epoch");

    // THE GENERATION CONTINUES THE SINGLE FILE'S: four writes made it 4, and
    // each partition advances from there by its own keys, so no reader keyed
    // on a generation sees the census go backwards across the migration.
    let legacy_generation = Manifest::open_image(Vendor::Gdfl, &before)
        .expect("the single file")
        .header()
        .generation;
    assert_eq!(legacy_generation, 4);
    assert_eq!(fno.header().generation, legacy_generation + 1);
    assert_eq!(idx.header().generation, legacy_generation + 1);
    assert_eq!(
        cash.header().generation,
        legacy_generation,
        "no key, no step"
    );

    // Twice is refused, by name, and changes nothing; `--seal` confirms the
    // seal and writes nothing.
    let mut twice = String::new();
    assert_eq!(migrate_with(&["gdfl"], &store, &mut twice), FAILED);
    assert!(twice.contains("already version 4"), "{twice}");
    assert_eq!(std::fs::read(&superseded).expect("kept"), before);
    let mut sealed = String::new();
    assert_eq!(
        migrate_with(&["gdfl", "--seal"], &store, &mut sealed),
        OK,
        "{sealed}"
    );
    assert!(
        sealed.contains("CENSUS SEALED") && sealed.contains("already sealed, nothing was written"),
        "{sealed}"
    );
    let mut unmigrated = String::new();
    assert_eq!(
        migrate_with(&["zerodha", "--seal"], &store, &mut unmigrated),
        FAILED
    );
    assert!(unmigrated.contains("CENSUS SEAL REFUSED"), "{unmigrated}");
    let mut misused = String::new();
    assert_eq!(
        migrate_with(&["gdfl", "--sealed"], &store, &mut misused),
        MISUSED
    );
}

/// A RECOUNT AFTER THE MIGRATION writes the FNO partition's NEXT epoch whole,
/// never the single file, and keeps epoch 000001 byte for byte; an ordinary
/// record then appends to that epoch; and a pre-version-4 writer that appends
/// to the single file afterwards is named (D-3316).
#[test]
fn a_recount_after_the_migration_writes_the_next_epoch_never_the_file() {
    let scratch = Scratch::new("migrated-recount");
    let root = scratch.0.clone();
    let (nov, dec) = (ym(2019, 11), ym(2019, 12));
    let before = three_writes_and_an_index_month(&root);
    let legacy = manifest_path(&root, Vendor::Gdfl);
    let store = || Ok(root.clone());
    let mut out = String::new();
    assert_eq!(migrate_with(&["gdfl"], &store, &mut out), OK, "{out}");

    let first_epoch = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
    let first_bytes = std::fs::read(&first_epoch).expect("epoch 1");
    write_option(&root, dec, &bars(dec, 26, 0, 4));
    let mut counted = String::new();
    assert_eq!(
        recount_with(
            &["gdfl", "--from", "2019-12-01"],
            &store,
            None,
            &mut counted
        ),
        OK,
        "{counted}"
    );
    assert!(
        counted.contains("one directory of epochs per segment, version 4"),
        "{counted}"
    );
    assert!(counted.contains("(epoch 000002)"), "{counted}");
    assert!(
        counted.contains("compacted (recount) epoch 000001 -> 000002: 1 -> 2 entries"),
        "{counted}"
    );
    let (epoch, fno) = partition(&root, Segment::Fno);
    assert_eq!(epoch, 2);
    assert_eq!(fno.held(&key(dec)).map(|held| held.entry.rows), Some(4));
    assert_eq!(
        (fno.entries(), fno.keys(), fno.total_rows()),
        (2, 2, 10),
        "one row per key"
    );
    assert_eq!(
        std::fs::read(&first_epoch).expect("epoch 1"),
        first_bytes,
        "kept"
    );
    let superseded = census_store::superseded_path(&root, Vendor::Gdfl);
    let stub = census_store::sealed_stub(Vendor::Gdfl);
    assert_eq!(std::fs::read(&superseded).expect("kept"), before);
    assert_eq!(std::fs::read(&legacy).expect("the seal"), stub);
    // A second recount changes nothing, so it installs no epoch.
    let mut again = String::new();
    assert_eq!(
        recount_with(&["gdfl", "--from", "2019-12-01"], &store, None, &mut again),
        OK,
        "{again}"
    );
    assert!(again.contains("recorded 0, already exact 1"), "{again}");
    assert_eq!(
        census_store::current_epoch(&root, Vendor::Gdfl, Segment::Fno).expect("listed"),
        Some(2)
    );
    // And an ordinary record of an option row now lands in the partition.
    assert_eq!(
        pull::ingest::record_held(
            &root,
            Vendor::Gdfl,
            &[row(nov, 8, at(nov, 13, 0), at(nov, 14, 1))]
        ),
        None
    );
    let (_, fno) = partition(&root, Segment::Fno);
    assert_eq!(fno.held(&key(nov)).map(|held| held.entry.rows), Some(8));
    assert_eq!(std::fs::read(&superseded).expect("kept"), before);
    assert_eq!(std::fs::read(&legacy).expect("the seal"), stub);
    // A writer that is not this build replacing the seal is named.
    let mut grown = before.clone();
    grown.extend_from_slice(&[0u8; 128]);
    std::fs::write(&legacy, &grown).expect("a stale writer wrote");
    let mut warned = String::new();
    let _ = recount_with(&["gdfl", "--from", "2019-12-01"], &store, None, &mut warned);
    assert!(
        warned.contains("WARNING") && warned.contains("no longer holds the seal"),
        "{warned}"
    );
}

/// The verbs refuse what they do not understand with MISUSED, and work they
/// could not do with FAILED.
#[test]
fn the_verbs_refuse_misuse_and_failure_with_different_codes() {
    let scratch = Scratch::new("refusals");
    let root = scratch.0.clone();
    let store = || Ok(root.clone());
    let mut out = String::new();
    assert_eq!(migrate_with(&["gdfl", "extra"], &store, &mut out), MISUSED);
    let mut out = String::new();
    assert_eq!(migrate_with(&["nobody"], &store, &mut out), MISUSED);
    assert!(out.contains("not a feed"), "{out}");
    let mut out = String::new();
    assert_eq!(recount_with(&["nobody"], &store, None, &mut out), MISUSED);
    let unavailable = || Err::<PathBuf, String>("the store root is not there".to_owned());
    let mut out = String::new();
    assert_eq!(
        recount_with(
            &["gdfl", "--from", "2019-11-14"],
            &unavailable,
            None,
            &mut out
        ),
        FAILED
    );
    assert!(
        out.contains("CENSUS RECOUNT REFUSED: the store root is not there"),
        "{out}"
    );
    let mut out = String::new();
    assert_eq!(migrate_with(&["gdfl"], &unavailable, &mut out), FAILED);
    assert!(out.contains("CENSUS MIGRATE REFUSED"), "{out}");
    // An empty store migrates to empty epochs.
    let mut out = String::new();
    assert_eq!(migrate_with(&["zerodha"], &store, &mut out), OK, "{out}");
    assert!(out.contains("did not exist"), "{out}");
    assert!(vendor_dir(&root, Vendor::Zerodha).is_dir());
    // A journal line this build does not know stops the recount by name.
    std::fs::create_dir_all(journal(&root).parent().expect("imports")).expect("imports");
    std::fs::write(journal(&root), "something else entirely\n").expect("a foreign journal");
    let mut out = String::new();
    assert_eq!(
        recount_with(&["gdfl", "--from", "2019-11-14"], &store, None, &mut out),
        FAILED
    );
    assert!(out.contains("not a journal line this build knows"), "{out}");
}
