//! THE OPERATOR'S SEQUENCE, END TO END, ON A SCRATCH STORE: an import stopped
//! by its census, the census migrated to version 4, the refused days recounted
//! from the bar headers, and the import resumed (D-3316 items 6 and 8, D-3317
//! items 5 and 7; DB-058, DB-G01, DB-G09, DB-G49, DB-G60, DB-G78).
//!
//! Each half is proved on its own elsewhere — the recount's journal lines in
//! `cli`'s `census_recount_tests`, the import's loader in `gdfl_import_tests`.
//! What neither proves is that the two modules AGREE: that the `run ...
//! source=recount` and `recounted` lines `pull::ingest::census_store` writes
//! are lines the import's own journal loader accepts and reads as done, that a
//! resumed import appends to the migrated partition rather than to the frozen
//! single file, and that afterwards the census and the bar files say the same
//! thing. A loader that refused the recount's lines would stop every later run
//! of the import by name; one that read them as anything but done would import
//! the recounted days again. This is that proof.
//!
//! Every value is invented at run time; no vendor row is quoted.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail; the operator's sequence is one test, \
              each step asserting on the state the one before it left"
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use pull::gdfl_import::{
    DayRead, ImportKind, ImportRefusal, Run, Tick, TickFile, drive, journal_path,
};
use pull::ingest::census_store::{self, Location, Span};
use pull::manifest::manifest_path;
use pull::session::Day;

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!(
        "brutex-census-resume-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("a scratch root");
    Scratch(dir)
}

fn day(d: u8) -> Day {
    Day::new(2024, 4, d).expect("a real April 2024 day")
}

/// One invented share's day: four traded rows, three of them in session, at
/// invented prices.
fn file() -> TickFile {
    TickFile {
        symbol: "RELIANCE".to_owned(),
        contract: None,
        name: "RELIANCE.invented".to_owned(),
        ticks: [NINE_FIFTEEN, NINE_FIFTEEN + 61, CLOSE - 1, CLOSE + 90]
            .into_iter()
            .map(|sod| Tick {
                sod,
                ltp: 123_400 + i64::from(sod % 9),
                ltq: 3,
                oi: None,
            })
            .collect(),
    }
}

/// The invented share's one CASH row in a census, as `(rows, last stamp)`.
fn cash_row(root: &Path) -> Vec<(u64, i64)> {
    census_store::read_vendor(root, Vendor::Gdfl)
        .expect("the census reads")
        .iter()
        .filter(|read| read.segment.is_none_or(|segment| segment == Segment::Cash))
        .flat_map(|read| {
            let census = &read.census;
            census
                .held_keys()
                .filter(|key| key.segment == Segment::Cash)
                .filter_map(|key| census.held(key))
                .map(|held| (held.entry.rows, held.entry.last_ts_micros))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn a_census_refused_day_recounted_after_the_migration_is_done_for_the_resumed_import() {
    let store = scratch("resume");
    let root = store.0.as_path();
    let run = |from: u8, to: u8| Run {
        kind: ImportKind::Stocks,
        from: day(from),
        to: day(to),
        only: &[],
        store_root: root,
    };
    let one_file = |sink: &mut dyn FnMut(TickFile)| {
        sink(file());
        DayRead {
            held: true,
            ..DayRead::default()
        }
    };

    // ── Day one imports and is counted in the single file ─────────────────
    let first = drive(&run(1, 1), |_, sink| one_file(sink)).expect("day one imports");
    assert_eq!(first.days_imported, 1);
    let [(rows_one, _)] = cash_row(root)[..] else {
        panic!("one CASH row after day one: {:?}", cash_row(root));
    };
    assert!(rows_one > 0, "day one wrote bars");

    // ── Day two: its bars land and the census refuses; the run stops ──────
    // The single file is swapped for a directory, which no census read
    // accepts, so the refusal is the census's own and the bars still land.
    let single = manifest_path(root, Vendor::Gdfl);
    let kept = single.with_extension("man.kept");
    std::fs::rename(&single, &kept).expect("set aside");
    std::fs::create_dir(&single).expect("a census that is a directory");
    let mut read = Vec::new();
    let stopped = drive(&run(2, 3), |on, sink| {
        read.push(on);
        one_file(sink)
    })
    .unwrap_err();
    assert_eq!(read, vec![day(2)], "the run stops at the refused day");
    let ImportRefusal::CensusRefused { day: at, .. } = &stopped else {
        panic!("{stopped:?}");
    };
    assert_eq!(*at, day(2));
    std::fs::remove_dir(&single).expect("the directory removed");
    std::fs::rename(&kept, &single).expect("the single file put back");
    let journal = std::fs::read_to_string(journal_path(root)).expect("the journal");
    assert!(
        journal
            .lines()
            .any(|line| line.starts_with("incomplete stocks 2024-04-02 * ")
                && line.contains(" stage=census ")),
        "{journal}"
    );
    assert_eq!(
        cash_row(root),
        vec![(rows_one, cash_row(root)[0].1)],
        "the census still counts day one only"
    );
    let single_bytes = std::fs::read(&single).expect("the single file");

    // ── WP-80 step 1: the migration ───────────────────────────────────────
    let migrated = census_store::migrate(root, Vendor::Gdfl).expect("the census migrates");
    assert_eq!(migrated.partitions.len(), 3);
    assert!(matches!(
        census_store::locate(root, Vendor::Gdfl),
        Ok(Location::Partitioned(_))
    ));

    // ── WP-80 step 2: the recount of the refused day, no vendor read ──────
    let found = census_store::recount(
        root,
        Vendor::Gdfl,
        Span {
            from: day(2),
            to: Some(day(2)),
        },
        Some("0123abcd"),
    )
    .expect("recounted");
    assert!(found.clean(), "{found:?}");
    let closed = found.journal.as_ref().expect("the gdfl journal");
    let keys: Vec<&str> = closed.closed.iter().map(|one| one.key.as_str()).collect();
    assert_eq!(keys, ["stocks 2024-04-02 *"]);
    let [(rows_two, _)] = cash_row(root)[..] else {
        panic!("one CASH row after the recount: {:?}", cash_row(root));
    };
    assert_eq!(rows_two, 2 * rows_one, "the census now counts both days");

    // ── WP-80 step 3: the import resumes ──────────────────────────────────
    // Its loader reads the recount's `run` and `recounted` lines without
    // refusing them, and takes the recounted day as done: only day three is
    // read.
    let mut read = Vec::new();
    let resumed = drive(&run(1, 3), |on, sink| {
        read.push(on);
        one_file(sink)
    })
    .expect("the import resumes over the recount's lines");
    assert_eq!(read, vec![day(3)], "days one and two are done");
    assert_eq!(resumed.days_skipped, 2);
    assert_eq!(resumed.days_imported, 1);
    let [(rows_three, _)] = cash_row(root)[..] else {
        panic!("one CASH row after day three: {:?}", cash_row(root));
    };
    assert_eq!(rows_three, 3 * rows_one, "day three went to the partition");

    // The frozen single file took no write after the migration.
    assert_eq!(
        std::fs::read(&single).expect("the single file"),
        single_bytes
    );
    assert_eq!(census_store::legacy_drift(root, Vendor::Gdfl), None);

    // And the census says what the bar headers say: a recount of every day
    // records nothing and closes nothing.
    let again = census_store::recount(
        root,
        Vendor::Gdfl,
        Span {
            from: day(1),
            to: None,
        },
        None,
    )
    .expect("recounted again");
    assert!(again.clean(), "{again:?}");
    assert_eq!(
        again
            .cycles
            .iter()
            .map(|cycle| cycle.recorded)
            .sum::<usize>(),
        0,
        "{again:?}"
    );
    assert_eq!(
        again.cycles.iter().map(|cycle| cycle.exact).sum::<usize>(),
        1,
        "{again:?}"
    );
    assert!(
        again
            .journal
            .as_ref()
            .is_some_and(|journal| journal.closed.is_empty() && journal.run.is_none()),
        "{again:?}"
    );
}
