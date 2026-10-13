//! A GDFL DAY ITS MONTH LACKS, WHERE NO APPEND CAN LAND IT, LANDS AS A NEW
//! GENERATION OF THE MONTH (D-3350; DB-N26, WP-12d).
//!
//! On 10 Oct 2026 the import refused every NIFTY and BANKNIFTY file of the 12
//! index days WP-12a admitted (8 Muhurat sessions, 2 DR drills, 2021-02-24,
//! 2022-03-07): "batch record 0 is inside the held range at a stamp the month
//! never held". Each day lies inside a month already held past it, which the
//! append-only month cannot take. These tests build that shape from invented
//! files — a month holding 2024-04-01 and 2024-04-03, then 2024-04-02 — and
//! read back what every reader opens: the bars, the served generation, the
//! census row, the `.tix` lookup, the `.prv`, the import journal and the
//! repair journal, beside a store that imported the same days in order.
//!
//! They also hold the door to its rule from the other side: a refusal that is
//! not the month's shape — a month a reader holds locked, a held day read
//! differently, a revision tree that cannot be written — stays a refusal, by
//! name, and never becomes a generation.
//!
//! Every value is invented at run time; no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file too).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::{csv, row, scratch};
use crate::ingest::{Filed, repair_rows};

/// The NIFTY one-second month of April 2024.
fn nifty_april() -> StorePath<'static> {
    month_of_key("INDEX", "NIFTY", d1())
}

/// The one-second month of `symbol` that holds `on`.
fn month_of_key(segment: &'static str, symbol: &'static str, on: Day) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: on.year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The store's id of `symbol`, as the import derives it.
fn id_of(symbol: &str) -> u32 {
    symbol_id_of(symbol)
}

/// Every bar of the month at `path`, as every reader opens it.
fn served_bars(store: &Path, path: StorePath<'_>, symbol: &str) -> Vec<Bar> {
    let file = BarFile::open_existing(store, path, id_of(symbol)).unwrap();
    (0..file.header().n_valid)
        .map(|index| file.read_record(index).unwrap())
        .collect()
}

/// The generation of the month at `path` every reader opens: `0` the
/// original.
fn generation(store: &Path, path: StorePath<'_>) -> u32 {
    store::generation::served(store, path).unwrap().ordinal()
}

/// Every file under `dir`, or nothing when it does not exist.
fn tree_or_empty(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    if dir.exists() {
        tree(dir)
    } else {
        BTreeMap::new()
    }
}

/// The repair journal's lines.
fn repair_lines(store: &Path) -> Vec<String> {
    std::fs::read_to_string(store.join("audit").join("gdfl.repairs"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The repair journal's lines whose first word is `verb`.
fn repair_lines_of(store: &Path, verb: &str) -> Vec<String> {
    repair_lines(store)
        .into_iter()
        .filter(|line| line.split(' ').next() == Some(verb))
        .collect()
}

/// The IST day a bar is stamped on.
fn day_of(bar: &Bar) -> i64 {
    (bar.ts_micros / 1_000_000 + IST_OFFSET_SECS).div_euclid(86_400)
}

/// The census row of NIFTY's April month, as every census reader loads it.
fn census_row(store: &Path) -> Held {
    let key = crate::manifest::EntryKey {
        contract: None,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        symbol: brutex_core::symbol::Symbol::new("NIFTY").unwrap(),
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
    };
    crate::ingest::census_store::read_vendor(store, Vendor::Gdfl)
        .unwrap()
        .iter()
        .find_map(|read| read.census.held(&key))
        .expect("the census counts NIFTY's April")
}

/// A store holding 2024-04-01 and 2024-04-03 of `kind`, each imported in
/// its own run, and a store holding 2024-04-01..03 imported in order, both
/// from `root`'s tick store.
fn holed_and_whole(root: &Path, kind: ImportKind) -> (PathBuf, PathBuf) {
    holed_and_whole_in(root, kind, root)
}

/// [`holed_and_whole`] with both stores under `below`.
fn holed_and_whole_in(root: &Path, kind: ImportKind, below: &Path) -> (PathBuf, PathBuf) {
    let (holed, whole) = (below.join("holed"), below.join("whole"));
    for on in [d1(), d3()] {
        let got = import(root, Src::Store, kind, on, on, &holed);
        assert!(got.failures.is_empty(), "{:?}", got.failures);
    }
    let got = import(root, Src::Store, kind, d1(), d3(), &whole);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    (holed, whole)
}

/// **THE DB-N26 SHAPE, LANDED.** A month holding 2024-04-01 and 2024-04-03
/// takes 2024-04-02: no failure, the day closed `done`, each index file's
/// month a new generation holding exactly the bars an in-order import holds,
/// and every reader opens it — the bars, the served generation, the census
/// row, the `.tix` lookup. The original month's files and an untouched month
/// are byte-identical, and the repair journal names the run, before the
/// repair and after it.
#[test]
fn a_day_a_held_month_lacks_lands_as_a_new_generation_every_reader_opens() {
    let root = scratch("wp12d-backfill");
    let may = day(2024, 5, 2);
    world(&root, &[d1(), d2(), d3(), may]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    let untouched = import(&root, Src::Store, ImportKind::Indices, may, may, &holed);
    assert!(untouched.failures.is_empty(), "{:?}", untouched.failures);
    // Every file of every month the store holds, April's originals and May.
    let months_before = tree(&holed.join("bars"));
    let held_before = bars(&holed, "INDEX", "NIFTY", None);

    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert!(
        got.files >= 2,
        "NIFTY and BANKNIFTY at least: {}",
        got.files
    );
    assert_eq!(
        got.months_repaired, got.files,
        "one new generation per file"
    );
    assert_eq!(
        got.seconds_committed, got.seconds,
        "every bar of the day added"
    );
    assert!(got.seconds > 0);
    let journal = std::fs::read_to_string(journal_path(&holed)).unwrap();
    assert!(journal.contains("\ndone indices 2024-04-02 *"), "{journal}");
    assert!(
        !journal.contains("incomplete indices 2024-04-02"),
        "{journal}"
    );

    // EVERY READER OPENS THE NEW GENERATION, and it holds what an in-order
    // import holds, bar for bar: the held days kept, the day between them.
    assert_eq!(generation(&holed, nifty_april()), 1);
    let now = bars(&holed, "INDEX", "NIFTY", None);
    assert_eq!(now, bars(&whole, "INDEX", "NIFTY", None));
    assert_eq!(
        bars(&holed, "INDEX", "BANKNIFTY", None),
        bars(&whole, "INDEX", "BANKNIFTY", None)
    );
    let day_two = i64::from(d2().days_from_epoch());
    let added: Vec<&Bar> = now.iter().filter(|bar| day_of(bar) == day_two).collect();
    assert_eq!(added.len(), now.len() - held_before.len());
    assert!(
        held_before.iter().all(|bar| now.contains(bar)),
        "no held bar dropped or changed"
    );
    let file = BarFile::open_existing(&holed, nifty_april(), id_of("NIFTY")).unwrap();
    // THE NEW GENERATION CARRIES A `.tix` EXACTLY WHEN IT REACHES THE
    // SMALL-MONTH THRESHOLD (D-4933); under it, it is bisected by rule.
    let small = store::time_index::below_threshold(file.header().n_valid);
    assert_eq!(
        file.time_lookup(),
        if small {
            store::file::TimeLookup::Bisection(store::time_index::Why::BelowThreshold)
        } else {
            store::file::TimeLookup::Indexed
        }
    );
    let first_added = added.first().unwrap().ts_micros;
    let last_added = added.last().unwrap().ts_micros;
    let at = file.first_at_or_after(first_added).unwrap();
    assert_eq!(file.read_record(at).unwrap().ts_micros, first_added);
    let revision_tix =
        nifty_april()
            .with_file(FileKind::TimeIndex)
            .to_path_buf(&store::repair::revision_root(
                &holed,
                store::repair::Revision::new(1).unwrap(),
            ));
    assert_eq!(revision_tix.exists(), !small, "{}", revision_tix.display());

    // THE CENSUS ROW IS THE SERVED GENERATION'S.
    let row = census_row(&holed);
    let header = file.header();
    assert_eq!(row.entry.rows, header.n_valid);
    assert_eq!(row.entry.first_ts_micros, header.first_ts_micros);
    assert_eq!(row.entry.last_ts_micros, header.last_ts_micros);
    assert_eq!(row.entry.rows, census_row(&whole).entry.rows);

    // NEVER IN PLACE, and nothing else moved: every file under `bars/` — the
    // originals of the repaired months and the untouched May — is the byte
    // it was; the new generations are under `bar-revisions-v1/`.
    assert_eq!(tree(&holed.join("bars")), months_before);
    assert!(
        months_before
            .keys()
            .any(|path| path.ends_with("NIFTY/1s/2024-05.bin"))
    );

    // JOURNALED, BEFORE AND AFTER: one `repairing` line and one `repair`
    // line per repaired month, the first written before anything was
    // published, both naming the run.
    let lines = repair_lines(&holed);
    assert_eq!(lines.len(), 2 * got.months_repaired, "{lines:?}");
    let run = run_ids(&journal).into_iter().max().unwrap();
    let reason = format!("reason=gdfl-import_indices_2024-04-02_NIFTY_50.NSE_IDX.csv_run:{run}:_");
    let intent = lines
        .iter()
        .position(|line| line.starts_with("repairing NSE-INDEX-NIFTY 1s 2024-04 v=1 "))
        .expect("NIFTY's repairing line");
    let repair = lines
        .iter()
        .position(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 v=1 "))
        .expect("NIFTY's repair line");
    assert!(intent < repair, "{lines:?}");
    for field in [
        "from_generation=0".to_owned(),
        format!("offered={}", added.len()),
        format!("first={first_added}"),
        format!("last={last_added}"),
        reason.clone(),
    ] {
        assert!(
            lines[intent]
                .split(' ')
                .any(|have| have.starts_with(&field)),
            "{field} in {}",
            lines[intent]
        );
    }
    for field in [
        "from_generation=0".to_owned(),
        "to_generation=1".to_owned(),
        format!("held={}", held_before.len()),
        format!("added={}", added.len()),
        format!("rows={}", header.n_valid),
        "revision=created".to_owned(),
        "skipped=-".to_owned(),
        reason,
    ] {
        assert!(
            lines[repair]
                .split(' ')
                .any(|have| have.starts_with(&field)),
            "{field} in {}",
            lines[repair]
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// **RESUMABLE, AND A RE-RUN WRITES NOTHING.** A re-run over the back-filled
/// day skips it. A run KILLED AFTER THE PROMOTION — before its `repair` line
/// and before the day's closing line — imports the day again, finds every bar
/// held in the served generation, writes nothing, publishes no third
/// generation and closes the day `done`; and the generation it left served is
/// named in the repair journal all the same, by the `repairing` line written
/// before it was published.
#[test]
fn a_rerun_writes_nothing_and_a_run_killed_after_the_promotion_resumes_without_another_generation()
{
    let root = scratch("wp12d-resume");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, _whole) = holed_and_whole(&root, ImportKind::Indices);
    let first = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let after = tree(&holed);

    let again = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert_eq!((again.days_skipped, again.months_repaired), (1, 0));
    assert_eq!(tree(&holed), after, "a re-run changes no byte");

    // THE KILL: neither the `repair` lines nor the day's closing line were
    // ever written. What the journals hold is what was written before.
    let journal = journal_path(&holed);
    let text = std::fs::read_to_string(&journal).unwrap();
    let mut cut = String::with_capacity(text.len());
    for line in text
        .lines()
        .filter(|line| !line.starts_with("done indices 2024-04-02 "))
    {
        cut.push_str(line);
        cut.push('\n');
    }
    assert_ne!(cut, text);
    std::fs::write(&journal, cut).unwrap();
    let intents = repair_lines_of(&holed, "repairing");
    assert_eq!(intents.len(), first.months_repaired, "{intents:?}");
    let mut kept = intents.join("\n");
    kept.push('\n');
    std::fs::write(holed.join("audit").join("gdfl.repairs"), kept).unwrap();

    let resumed = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.resumed, vec![d2()]);
    assert_eq!(
        (resumed.seconds_committed, resumed.months_repaired),
        (0, 0),
        "every bar was already served"
    );
    assert!(resumed.seconds > 0);
    assert_eq!(generation(&holed, nifty_april()), 1);
    assert!(!holed.join("bar-revisions-v1").join("2").exists());
    // NO REPAIR, NO NEW LINE; and the served generation is not unnamed.
    assert_eq!(repair_lines(&holed), intents);
    let run = run_ids(&text).into_iter().max().unwrap();
    assert!(
        intents.iter().any(|line| line
            .starts_with("repairing NSE-INDEX-NIFTY 1s 2024-04 v=1 from_generation=0 ")
            && line.contains(&format!("_run:{run}:_"))),
        "{intents:?}"
    );
    let text = std::fs::read_to_string(&journal).unwrap();
    assert!(text.contains("\ndone indices 2024-04-02 *"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A crash BETWEEN PUBLICATION AND PROMOTION: the revision a repair would
/// publish is complete on disk and no reader serves it. The import finds it
/// at its ordinal, verifies it byte for byte, promotes it and says so
/// (`revision=reused`), and every reader then opens it.
#[test]
fn a_revision_published_and_never_promoted_is_promoted_by_the_next_import() {
    let root = scratch("wp12d-unpromoted");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    let header = BarFile::open_existing(&holed, nifty_april(), id_of("NIFTY"))
        .unwrap()
        .header();
    let merged = bars(&whole, "INDEX", "NIFTY", None);
    let published = store::repair::publish(
        &holed,
        nifty_april(),
        id_of("NIFTY"),
        store::repair::Revision::new(1).unwrap(),
        header,
        &merged,
    )
    .unwrap();
    assert_eq!(published, store::repair::Published::Created);
    assert_eq!(
        generation(&holed, nifty_april()),
        0,
        "published, not served"
    );

    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(generation(&holed, nifty_april()), 1);
    assert_eq!(bars(&holed, "INDEX", "NIFTY", None), merged);
    let nifty = repair_lines(&holed)
        .into_iter()
        .find(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 "))
        .unwrap();
    assert!(nifty.contains(" to_generation=1 "), "{nifty}");
    assert!(nifty.contains(" revision=reused "), "{nifty}");
    std::fs::remove_dir_all(root).unwrap();
}

/// **KILLED MID-WRITE.** A repair that died while writing its revision left
/// the ordinal reserved, a bar file part-written and no receipt. The next
/// import never opens, reuses or overwrites it: it steps over the ordinal,
/// publishes the month at the next one, serves that, and the journal names
/// the ordinal stepped over.
#[test]
fn a_revision_torn_mid_write_is_stepped_over_never_reused_and_named() {
    let root = scratch("wp12d-torn");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    let first = store::repair::revision_root(&holed, store::repair::Revision::new(1).unwrap());
    let torn_bars = nifty_april().to_path_buf(&first);
    std::fs::create_dir_all(torn_bars.parent().unwrap()).unwrap();
    std::fs::write(torn_bars.with_extension("reserved-v1"), b"").unwrap();
    std::fs::write(&torn_bars, vec![0x5a_u8; 4_099]).unwrap();
    let torn = tree(&first);

    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.months_repaired, got.files);
    assert_eq!(generation(&holed, nifty_april()), 2);
    assert_eq!(
        bars(&holed, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );
    let now = tree(&first);
    for (path, bytes) in &torn {
        assert_eq!(now.get(path), Some(bytes), "{}", path.display());
    }
    let nifty = repair_lines(&holed)
        .into_iter()
        .find(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 "))
        .unwrap();
    assert!(
        nifty.contains(" from_generation=0 to_generation=2 "),
        "{nifty}"
    );
    assert!(nifty.contains(" skipped=1 "), "{nifty}");
    // BANKNIFTY's month had no torn ordinal: it is generation 1.
    assert_eq!(
        generation(&holed, month_of_key("INDEX", "BANKNIFTY", d1())),
        1
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **A REVISION TREE THAT CANNOT BE WRITTEN** (a full or read-only disk
/// answers the same way: the host refuses the write). The back-fill is
/// refused by name, nothing is served but the original, the original's files
/// are byte-identical, both journal lines say so, and the day stays
/// `incomplete`; once the host takes the write, the same command lands it.
#[test]
fn a_revision_tree_the_host_refuses_is_named_and_the_day_lands_once_it_can_be_written() {
    let root = scratch("wp12d-unwritable");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    // A FILE WHERE THE REVISION TREE BELONGS: no directory can be made there.
    let blocked = holed.join("bar-revisions-v1");
    std::fs::write(&blocked, b"not a directory").unwrap();
    let originals = tree(&holed.join("bars"));

    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);

    assert_eq!(got.months_repaired, 0);
    assert_eq!(got.seconds_committed, 0);
    assert_eq!(got.failures.len(), got.files, "{:?}", got.failures);
    for failure in &got.failures {
        assert!(
            failure.why.contains("the versioned repair refused it too")
                && failure.why.contains("bar-revisions-v1"),
            "{}",
            failure.why
        );
    }
    assert_eq!(got.closed_incomplete, vec![d2()]);
    assert_eq!(generation(&holed, nifty_april()), 0);
    assert_eq!(tree(&holed.join("bars")), originals);
    assert_eq!(std::fs::read(&blocked).unwrap(), b"not a directory");
    let lines = repair_lines(&holed);
    assert_eq!(lines.len(), 2 * got.files, "{lines:?}");
    assert_eq!(repair_lines_of(&holed, "repairing").len(), got.files);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("refused NSE-INDEX-NIFTY 1s 2024-04 v=1 why=")),
        "{lines:?}"
    );
    assert!(repair_lines_of(&holed, "repair").is_empty());

    // THE HOST TAKES THE WRITE: the same command, and the day lands.
    std::fs::remove_file(&blocked).unwrap();
    let landed = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert!(landed.failures.is_empty(), "{:?}", landed.failures);
    assert_eq!(landed.resumed, vec![d2()]);
    assert_eq!(landed.months_repaired, landed.files);
    assert_eq!(generation(&holed, nifty_april()), 1);
    assert_eq!(
        bars(&holed, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// NIFTY's 2024-04-01 file with every price one rupee above `world`'s, at the
/// seconds `world` prints and one more it never prints (09:15:30).
fn nifty_restated(d: Day) -> Vec<u8> {
    let stem = "NIFTY 50.NSE_IDX";
    let p = |v: u32| format!("{}.{:02}", 22_001 + v / 100, v % 100);
    let mut rows = vec![row(stem, d, NINE_FIFTEEN - 300, &p(1), 0, 0)];
    rows.push(row(stem, d, NINE_FIFTEEN, &p(5), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 2, &p(20), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 30, &p(40), 0, 0));
    for s in (60..22_500).step_by(997) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, &p(s % 500), 0, 0));
    }
    rows.push(row(stem, d, CLOSE + 60, &p(7), 0, 0));
    csv(&rows)
}

/// `world`'s own NIFTY file of `d` (`index_file` at its base), row for row,
/// with ONE MORE second it never prints, 09:16:01: every bar the two files
/// share is the same bar.
fn nifty_with_one_more_second(d: Day) -> Vec<u8> {
    let stem = "NIFTY 50.NSE_IDX";
    let p = |v: u32| format!("{}.{:02}", 22_000 + v / 100, v % 100);
    let mut rows = vec![row(stem, d, NINE_FIFTEEN - 300, &p(1), 0, 0)];
    rows.push(row(stem, d, NINE_FIFTEEN, &p(5), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(55), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(15), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 2, &p(20), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 1, &p(30), 0, 0));
    for s in (60..22_500).step_by(997) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, &p(s % 500), 0, 0));
        if s == 60 {
            rows.push(row(stem, d, NINE_FIFTEEN + 61, &p(77), 0, 0));
        }
    }
    rows.push(row(stem, d, CLOSE + 60, &p(7), 0, 0));
    csv(&rows)
}

/// A world under `root` holding one NIFTY index file, `bytes`, on `d`.
fn nifty_world(root: &Path, d: Day, bytes: Vec<u8>) {
    world_of(
        root,
        &[(
            d,
            Tree::Indices,
            vec![(entry_of(Tree::Indices, d, "NIFTY 50.NSE_IDX"), bytes)],
        )],
    );
}

/// **A HELD BAR IS NEVER CHANGED, AND NOTHING IS TRIED.** A file whose day
/// the month holds with other values — and a second the month lacks besides —
/// is refused by name, in the store's words and the door's: the month serves
/// its original generation, no repair is attempted (no journal line, no
/// revision tree), the original's files are byte-identical, and the day stays
/// `incomplete`.
#[test]
fn a_file_that_would_change_a_held_bar_is_refused_by_name_and_writes_nothing() {
    let root = scratch("wp12d-differs");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, _whole) = holed_and_whole(&root, ImportKind::Indices);
    let restated = root.join("restated");
    nifty_world(&restated, d1(), nifty_restated(d1()));
    let before = tree(&holed.join("bars"));

    let got = import_only(
        &restated,
        Src::Store,
        ImportKind::Indices,
        d1(),
        d1(),
        &holed,
        &["NIFTY".to_owned()],
    );

    assert_eq!(got.months_repaired, 0);
    assert_eq!(got.seconds_committed, 0);
    assert_eq!(got.failures.len(), 1, "{:?}", got.failures);
    let why = &got.failures[0].why;
    assert!(why.contains("restates the record held at"), "{why}");
    assert!(why.contains("not back-filled"), "{why}");
    assert!(
        why.contains("never completes, restates or mixes a held day"),
        "{why}"
    );
    assert_eq!(got.closed_incomplete, vec![d1()]);
    assert_eq!(generation(&holed, nifty_april()), 0);
    assert!(tree_or_empty(&holed.join("bar-revisions-v1")).is_empty());
    assert!(!holed.join("audit").join("gdfl.repairs").exists());
    assert_eq!(tree(&holed.join("bars")), before);
    std::fs::remove_dir_all(root).unwrap();
}

/// **A HELD DAY IS NEVER COMPLETED BY A BACK-FILL.** A file of a day the
/// month already holds, every shared bar the same and ONE second more, is
/// not "a day the month lacks": merging it would leave a month no single
/// reading of the day produces while its journal line says `done`. The
/// append-only month refuses the new second, the door refuses to repair, no
/// generation is published — in an original month and in a month already
/// served from a revision alike.
#[test]
fn a_held_day_read_with_one_more_second_is_refused_never_merged() {
    let root = scratch("wp12d-completes");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    let more = root.join("more");
    nifty_world(&more, d1(), nifty_with_one_more_second(d1()));
    // THE PREMISE, CHECKED: the new reading is the held day plus one bar.
    let alone = root.join("alone");
    let fresh = import(&more, Src::Store, ImportKind::Indices, d1(), d1(), &alone);
    assert!(fresh.failures.is_empty(), "{:?}", fresh.failures);
    let day_one = i64::from(d1().days_from_epoch());
    let held_day: Vec<Bar> = bars(&holed, "INDEX", "NIFTY", None)
        .into_iter()
        .filter(|bar| day_of(bar) == day_one)
        .collect();
    let offered = bars(&alone, "INDEX", "NIFTY", None);
    assert_eq!(offered.len(), held_day.len() + 1);
    assert!(held_day.iter().all(|bar| offered.contains(bar)));

    let only = ["NIFTY".to_owned()];
    for served in [0, 1] {
        if served == 1 {
            // The month now served from a revision: 2024-04-02 back-filled.
            let filled = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
            assert!(filled.failures.is_empty(), "{:?}", filled.failures);
            assert_eq!(generation(&holed, nifty_april()), 1);
        }
        let before = (tree(&holed.join("bars")), repair_lines(&holed));
        let revisions = tree_or_empty(&holed.join("bar-revisions-v1"));
        let got = import_only(
            &more,
            Src::Store,
            ImportKind::Indices,
            d1(),
            d1(),
            &holed,
            &only,
        );
        assert_eq!(got.months_repaired, 0, "served {served}");
        assert_eq!(got.seconds_committed, 0, "served {served}");
        assert_eq!(got.failures.len(), 1, "{:?}", got.failures);
        let why = &got.failures[0].why;
        assert!(why.contains("not back-filled"), "served {served}: {why}");
        assert!(
            why.contains("inside the offered span"),
            "served {served}: {why}"
        );
        assert_eq!(got.closed_incomplete, vec![d1()], "served {served}");
        assert_eq!(generation(&holed, nifty_april()), served);
        assert_eq!(
            (tree(&holed.join("bars")), repair_lines(&holed)),
            before,
            "served {served}: nothing written, nothing journaled"
        );
        assert_eq!(
            tree_or_empty(&holed.join("bar-revisions-v1")),
            revisions,
            "served {served}: no revision published"
        );
    }
    assert_eq!(
        bars(&holed, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **A LOCK IS NOT A HOLE.** A reader holds a month while the import appends
/// the day after its last: the append-only door refuses, "another writer
/// holds", and that is the whole answer — the day is a TAIL an append lands,
/// so nothing is repaired, no generation is published, and the next run,
/// with the reader gone, appends it to the original. (The door once handed
/// every refusal to the repair, which reads under a shared lock a reader
/// does not block: a passing lock would have turned a plain append into a
/// new generation, and every later day of the month into another.)
#[test]
fn a_month_a_reader_holds_is_refused_as_the_append_door_refuses_it_never_repaired() {
    let root = scratch("wp12d-locked");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let first = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let before = tree(&store.join("bars"));

    let reader = BarFile::open_existing(&store, nifty_april(), id_of("NIFTY")).unwrap();
    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    drop(reader);

    assert_eq!(got.months_repaired, 0);
    let nifty: Vec<&Failure> = got
        .failures
        .iter()
        .filter(|failure| failure.instrument.contains("NIFTY 50"))
        .collect();
    assert_eq!(nifty.len(), 1, "{:?}", got.failures);
    assert_eq!(
        got.failures.len(),
        1,
        "only the held month: {:?}",
        got.failures
    );
    assert!(
        nifty[0].why.contains("another writer holds"),
        "{}",
        nifty[0].why
    );
    assert!(!nifty[0].why.contains("back-filled"), "{}", nifty[0].why);
    assert!(!nifty[0].why.contains("repair"), "{}", nifty[0].why);
    assert_eq!(got.closed_incomplete, vec![d2()]);
    assert_eq!(generation(&store, nifty_april()), 0);
    assert!(!store.join("bar-revisions-v1").exists());
    assert!(!store.join("audit").join("gdfl.repairs").exists());
    let held: BTreeMap<PathBuf, Vec<u8>> = tree(&store.join("bars"))
        .into_iter()
        .filter(|(path, _)| path.to_string_lossy().contains("/NIFTY/"))
        .collect();
    for (path, bytes) in &held {
        assert_eq!(before.get(path), Some(bytes), "{}", path.display());
    }

    // THE READER GONE: the same command appends the day to the original.
    let again = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(again.months_repaired, 0);
    assert_eq!(generation(&store, nifty_april()), 0);
    assert!(!store.join("bar-revisions-v1").exists());
    let whole = root.join("whole");
    let in_order = import(&root, Src::Store, ImportKind::Indices, d1(), d2(), &whole);
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
    assert_eq!(tree(&store.join("bars")), tree(&whole.join("bars")));
    std::fs::remove_dir_all(root).unwrap();
}

/// **A READER DOES NOT STOP A BACK-FILL, AND KEEPS ITS OWN GENERATION.** A
/// reader holding the month's original while a day inside it is back-filled
/// goes on reading the bars it opened, byte for byte; a reader that opens
/// after the promotion reads the new generation.
#[test]
fn a_reader_open_across_a_back_fill_keeps_the_generation_it_opened() {
    let root = scratch("wp12d-reader");
    world(&root, &[d1(), d2(), d3()]);
    let (holed, whole) = holed_and_whole(&root, ImportKind::Indices);
    let held = bars(&holed, "INDEX", "NIFTY", None);

    let reader = BarFile::open_existing(&holed, nifty_april(), id_of("NIFTY")).unwrap();
    let got = import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &holed);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.months_repaired, got.files);
    let through_old: Vec<Bar> = (0..reader.header().n_valid)
        .map(|index| reader.read_record(index).unwrap())
        .collect();
    drop(reader);

    assert_eq!(through_old, held, "the reader's generation never moved");
    assert_eq!(generation(&holed, nifty_april()), 1);
    assert_eq!(
        bars(&holed, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **THE SPARSE-MONTH SHAPE (DB-B40, DB-B61, DB-B76): A MONTH THAT HOLDS ONLY
/// A LATER DAY TAKES EVERY OTHER DAY.** The store holds 2024-04-03 alone, as
/// the six alias indices hold only a special session in 67 months. The days
/// before it land (before the month's first bar, then between), and so does
/// the day after it — a tail, but of a revision, which is never appended to.
/// Each landing is one generation, the chain is journaled link by link, and
/// the month ends holding exactly what an in-order import holds.
#[test]
fn a_month_holding_only_a_later_day_takes_the_days_before_between_and_after() {
    let root = scratch("wp12d-sparse");
    world(&root, &[d1(), d2(), d3(), d4()]);
    let (sparse, whole) = (root.join("sparse"), root.join("whole"));
    let only_late = import(&root, Src::Store, ImportKind::Indices, d3(), d3(), &sparse);
    assert!(only_late.failures.is_empty(), "{:?}", only_late.failures);
    let originals = tree(&sparse.join("bars"));
    let in_order = import(&root, Src::Store, ImportKind::Indices, d1(), d4(), &whole);
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);

    for (on, served) in [(d1(), 1), (d2(), 2), (d4(), 3)] {
        let got = import(&root, Src::Store, ImportKind::Indices, on, on, &sparse);
        assert!(got.failures.is_empty(), "{on}: {:?}", got.failures);
        assert_eq!(got.months_repaired, got.files, "{on}");
        assert_eq!(got.seconds_committed, got.seconds, "{on}");
        assert_eq!(generation(&sparse, nifty_april()), served, "{on}");
    }

    for symbol in ["NIFTY", "BANKNIFTY"] {
        assert_eq!(
            bars(&sparse, "INDEX", symbol, None),
            bars(&whole, "INDEX", symbol, None),
            "{symbol}"
        );
    }
    assert_eq!(census_row(&sparse).entry, census_row(&whole).entry);
    assert_eq!(tree(&sparse.join("bars")), originals, "never in place");
    let chain: Vec<String> = repair_lines_of(&sparse, "repair")
        .into_iter()
        .filter(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 "))
        .collect();
    assert_eq!(chain.len(), 3, "{chain:?}");
    for (link, line) in chain.iter().enumerate() {
        assert!(
            line.contains(&format!(
                " from_generation={link} to_generation={} ",
                link + 1
            )),
            "{line}"
        );
    }
    let journal = std::fs::read_to_string(journal_path(&sparse)).unwrap();
    for on in ["2024-04-01", "2024-04-02", "2024-04-04"] {
        assert!(
            journal.contains(&format!("\ndone indices {on} *")),
            "{on}: {journal}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// The seconds of one invented index file of a special session: three rows
/// before `open`, `count` rows from `open` every seven seconds, and three
/// after `end`, so the reader's end check holds.
fn special_index_file(stem: &str, d: Day, base: u32, (open, end): (u32, u32)) -> Vec<u8> {
    let p = |v: u32| format!("{}.{:02}", base + v / 100, v % 100);
    let mut rows = Vec::new();
    for early in 0..3 {
        rows.push(row(stem, d, open - 300 + early, &p(early), 0, 0));
    }
    let mut at = open;
    while at < end {
        rows.push(row(stem, d, at, &p(at % 400), 0, 0));
        at += 7;
    }
    for late in 0..3 {
        rows.push(row(stem, d, end + 300 + late, &p(late), 0, 0));
    }
    csv(&rows)
}

/// **A SPECIAL SESSION, THE DAY DB-N26 IS ABOUT.** The 2025-10-21 Muhurat
/// (13:45 to 14:45, the calendar of record's window) was refused when its
/// month was imported and is admitted since D-3319. The month holds the
/// sessions either side; the Muhurat lands between them as a new generation,
/// only its window's seconds, for NIFTY and BANKNIFTY alike, and the month
/// equals an in-order import's.
#[test]
fn a_muhurat_session_inside_a_held_month_lands_inside_its_window() {
    let root = scratch("wp12d-muhurat");
    let (before, muhurat, after) = (day(2025, 10, 20), day(2025, 10, 21), day(2025, 10, 23));
    let window = (hms_at(13, 45, 0), hms_at(14, 45, 0));
    let indices = [("NIFTY 50.NSE_IDX", 25_800), ("NIFTY BANK.NSE_IDX", 58_000)];
    let regular = |d: Day| -> DayPlan {
        (
            d,
            Tree::Indices,
            indices
                .iter()
                .map(|&(stem, base)| (entry_of(Tree::Indices, d, stem), index_file(stem, d, base)))
                .collect(),
        )
    };
    let special: DayPlan = (
        muhurat,
        Tree::Indices,
        indices
            .iter()
            .map(|&(stem, base)| {
                (
                    entry_of(Tree::Indices, muhurat, stem),
                    special_index_file(stem, muhurat, base, window),
                )
            })
            .collect(),
    );
    world_of(&root, &[regular(before), special, regular(after)]);
    let (holed, whole) = (root.join("holed"), root.join("whole"));
    // AS THE MONTH WAS FIRST IMPORTED: the regular sessions, the Muhurat
    // refused. Then an in-order store, for what the month must equal.
    for on in [before, after] {
        let got = import(&root, Src::Store, ImportKind::Indices, on, on, &holed);
        assert!(got.failures.is_empty(), "{on}: {:?}", got.failures);
    }
    for on in [before, muhurat, after] {
        let got = import(&root, Src::Store, ImportKind::Indices, on, on, &whole);
        assert!(got.failures.is_empty(), "{on}: {:?}", got.failures);
    }
    let originals = tree(&holed.join("bars"));

    let got = import(
        &root,
        Src::Store,
        ImportKind::Indices,
        muhurat,
        muhurat,
        &holed,
    );

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.days_imported, got.days_refused), (1, 0));
    assert_eq!((got.files, got.months_repaired), (2, 2));
    assert_eq!(tree(&holed.join("bars")), originals, "never in place");
    let on_muhurat = i64::from(muhurat.days_from_epoch());
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let month = month_of_key("INDEX", symbol, muhurat);
        assert_eq!(generation(&holed, month), 1, "{symbol}");
        let now = served_bars(&holed, month, symbol);
        assert_eq!(now, served_bars(&whole, month, symbol), "{symbol}");
        let session: Vec<&Bar> = now.iter().filter(|bar| day_of(bar) == on_muhurat).collect();
        assert_eq!(session.len(), 515, "{symbol}: 3,600 seconds, one in seven");
        for bar in session {
            let second = second_of(bar.ts_micros, muhurat);
            assert!(
                (window.0..window.1).contains(&second),
                "{symbol}: a bar at second {second}"
            );
        }
    }
    let journal = std::fs::read_to_string(journal_path(&holed)).unwrap();
    assert!(journal.contains("\ndone indices 2025-10-21 *"), "{journal}");
    std::fs::remove_dir_all(root).unwrap();
}

/// **SHARES AND OPTION CONTRACTS TOO**, through the same door: a stock day
/// and an option day inside held months land as new generations holding
/// what an in-order import holds; an option's open interest is a
/// measurement, and the back-filled day's is kept as read.
#[test]
fn a_share_day_and_an_option_day_inside_held_months_land_too() {
    let root = scratch("wp12d-stocks-options");
    world(&root, &[d1(), d2(), d3()]);
    let shares: &[(&str, &str, Option<Contract>)] =
        &[("CASH", "RELIANCE", None), ("CASH", "SBIN", None)];
    let contracts: &[(&str, &str, Option<Contract>)] = &[("FNO", "NIFTY", Some(nifty_ce()))];
    for (kind, instruments) in [
        (ImportKind::Stocks, shares),
        (ImportKind::Options, contracts),
    ] {
        let below = root.join(kind.as_str());
        std::fs::create_dir_all(&below).unwrap();
        let (holed, whole) = holed_and_whole_in(&root, kind, &below);
        let got = import(&root, Src::Store, kind, d2(), d2(), &holed);
        assert!(got.failures.is_empty(), "{kind:?}: {:?}", got.failures);
        assert!(got.months_repaired > 0, "{kind:?}");
        assert_eq!(got.months_repaired, got.files, "{kind:?}");
        for &(segment, symbol, contract) in instruments {
            let now = bars(&holed, segment, symbol, contract);
            assert_eq!(now, bars(&whole, segment, symbol, contract), "{symbol}");
            if contract.is_some() {
                assert!(
                    now.iter().all(|bar| bar.open_interest != OI_NULL),
                    "an option's open interest is kept"
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// **NO `.prv` RECORD NAMES BARS IT CANNOT INDEX.** A run that names its
/// origin records each append beside its month (D-3334); a back-filled day
/// lands inside the new generation, where a record covering the tail of one
/// append cannot describe it, and a revision carries no `.prv`. So the run
/// writes none for it, closes the day `done` (no failure), leaves the
/// original's sidecar byte-identical, and the generation every reader opens
/// answers that it has no sidecar — `/provenance.json` then rebuilds the
/// month from the journal's day lines, labelled reconstructed.
#[test]
fn a_back_filled_landing_writes_no_prv_record_and_leaves_the_original_sidecar() {
    let root = scratch("wp12d-provenance");
    world(&root, &[d1(), d2(), d3()]);
    let store = root.join("S");
    for on in [d1(), d3()] {
        let got = run_from(
            &root,
            (Src::Zips, ImportKind::Options),
            (on, on),
            (&store, &[]),
            true,
        );
        assert!(got.failures.is_empty(), "{:?}", got.failures);
        assert_eq!(got.provenance_records, got.files);
    }
    assert_eq!(nifty_ce_sidecar(&store).unwrap().records().len(), 2);
    let prv = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: Some(nifty_ce()),
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Provenance,
    })
    .unwrap()
    .to_path_buf(&store);
    let sidecar = std::fs::read(&prv).unwrap();

    let got = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d2(), d2()),
        (&store, &[]),
        true,
    );

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.closed_incomplete, Vec::<Day>::new());
    assert_eq!(got.provenance_records, 0);
    assert_eq!(got.months_repaired, got.files);
    assert!(got.months_repaired > 0);
    assert_eq!(std::fs::read(&prv).unwrap(), sidecar, "never in place");
    assert!(
        nifty_ce_sidecar(&store).is_none(),
        "the served generation has no sidecar"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── the door itself, shape by shape ────────────────────────────────────────

/// One invented index bar at second `sod` of `d`.
fn bar_at(d: Day, sod: u32, price: i64) -> Bar {
    let secs = i64::from(d.days_from_epoch()) * 86_400 + i64::from(sod) - IST_OFFSET_SECS;
    Bar {
        ts_micros: secs * 1_000_000,
        open: price,
        high: price + 4,
        low: price - 4,
        close: price + 1,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// `count` invented index bars of `d`, one a second from 09:15:00 + `from`.
fn seconds_of(d: Day, from: u32, count: u32) -> Vec<Bar> {
    (0..count)
        .map(|s| bar_at(d, NINE_FIFTEEN + from + s, 9_000_000 + i64::from(s % 97)))
        .collect()
}

/// What the back-fill door did with one batch offered to NIFTY's April.
struct Offered {
    filed: Filed,
    committed: usize,
    failures: Vec<String>,
}

/// Offers `batch` to the store through the door the import files through.
fn offer(store: &Path, batch: &[Bar]) -> Offered {
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(d1(), day(2024, 5, 31)).unwrap(),
        granularity: Granularity::Second1,
    };
    let (done, filed) = repair_rows(
        batch,
        "NIFTY",
        "wp12d door",
        store,
        plan(ImportKind::Indices, &request, None),
        "wp12d door test",
    );
    Offered {
        filed,
        committed: done.bars_committed,
        failures: done
            .failures
            .into_iter()
            .map(|failure| failure.why)
            .collect(),
    }
}

/// **EVERY SHAPE A BATCH CAN MEET A MONTH IN, ONE BY ONE.** Empty; a month
/// that is not there; a tail; a duplicate; a single bar inside the held
/// range; a batch before the first held bar; a tail of a revision; a batch
/// that overlaps held bars; a batch out of order; a batch across two months.
/// Only the shapes no append can ever land publish a generation, and each
/// that refuses writes nothing.
#[test]
fn the_door_repairs_only_what_the_month_lacks_and_no_append_can_land() {
    let store = scratch("wp12d-door");
    let april = nifty_april();
    let revisions = |store: &Path| tree_or_empty(&store.join("bar-revisions-v1")).len();

    // EMPTY: nothing to file, nothing written, not even a directory.
    let empty = offer(&store, &[]);
    assert_eq!((empty.filed, empty.committed), (Filed::Appended, 0));
    assert!(empty.failures.is_empty(), "{:?}", empty.failures);
    assert!(!store.join("bars").exists());

    // A MONTH THAT IS NOT THERE: the append door creates it.
    let second_day = seconds_of(d2(), 0, 50);
    let created = offer(&store, &second_day);
    assert_eq!((created.filed, created.committed), (Filed::Appended, 50));
    // A TAIL: appended to the original.
    let fourth_day = seconds_of(d4(), 0, 40);
    let tail = offer(&store, &fourth_day);
    assert_eq!((tail.filed, tail.committed), (Filed::Appended, 40));
    // A DUPLICATE: held, nothing written.
    let again = offer(&store, &second_day);
    assert_eq!((again.filed, again.committed), (Filed::Appended, 0));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!((generation(&store, april), revisions(&store)), (0, 0));

    // A SINGLE BAR INSIDE THE HELD RANGE, at a stamp the month never held.
    let single = seconds_of(d3(), 600, 1);
    let inside = offer(&store, &single);
    assert_eq!((inside.filed, inside.committed), (Filed::NewGeneration, 1));
    assert_eq!(generation(&store, april), 1);
    // BEFORE THE FIRST HELD BAR.
    let first_day = seconds_of(d1(), 0, 30);
    let before = offer(&store, &first_day);
    assert_eq!((before.filed, before.committed), (Filed::NewGeneration, 30));
    assert_eq!(generation(&store, april), 2);
    // A TAIL OF A REVISION: never appended to, so one more generation.
    let fifth_day = seconds_of(d5(), 0, 20);
    let after = offer(&store, &fifth_day);
    assert_eq!((after.filed, after.committed), (Filed::NewGeneration, 20));
    assert_eq!(generation(&store, april), 3);
    // AND EACH AGAIN: every bar held, nothing written, no fourth generation.
    for batch in [&single, &first_day, &fifth_day] {
        let held = offer(&store, batch);
        assert_eq!((held.filed, held.committed), (Filed::Appended, 0));
        assert!(held.failures.is_empty(), "{:?}", held.failures);
    }
    assert_eq!(generation(&store, april), 3);
    let mut expected: Vec<Bar> = [&first_day, &second_day, &single, &fourth_day, &fifth_day]
        .into_iter()
        .flatten()
        .copied()
        .collect();
    expected.sort_by_key(|bar| bar.ts_micros);
    assert_eq!(served_bars(&store, april, "NIFTY"), expected);

    // WHAT REFUSES, AND WRITES NOTHING. A batch that overlaps held bars and
    // adds more; the same bars at other prices; a batch out of order; a
    // batch across two months.
    let published = revisions(&store);
    let journaled = repair_lines(&store);
    let mut wider = seconds_of(d2(), 0, 50);
    wider.extend(seconds_of(d2(), 50, 10));
    let mut restated = second_day.clone();
    restated[7].close += 1;
    let mut unordered = seconds_of(d3(), 0, 5);
    unordered.swap(1, 3);
    let mut two_months = seconds_of(d3(), 100, 2);
    two_months.extend(seconds_of(day(2024, 5, 2), 0, 2));
    for (name, batch, says) in [
        ("overlaps and adds", &wider, "not back-filled"),
        ("restates", &restated, "not back-filled"),
        ("two months", &two_months, "bars span 2024-04 to 2024-05"),
    ] {
        let refused = offer(&store, batch);
        assert_eq!(
            (refused.filed, refused.committed),
            (Filed::Appended, 0),
            "{name}"
        );
        assert_eq!(refused.failures.len(), 1, "{name}: {:?}", refused.failures);
        assert!(
            refused.failures[0].contains(says),
            "{name}: {}",
            refused.failures[0]
        );
        assert_eq!(revisions(&store), published, "{name}");
        assert_eq!(repair_lines(&store), journaled, "{name}");
    }
    // OUT OF ORDER: the store's own refusal, by name, and never a repair's
    // business: no line, no generation.
    let refused = offer(&store, &unordered);
    assert_eq!((refused.filed, refused.committed), (Filed::Appended, 0));
    assert_eq!(refused.failures.len(), 1, "{:?}", refused.failures);
    assert!(
        !refused.failures[0].contains("back-filled"),
        "{}",
        refused.failures[0]
    );
    assert_eq!(revisions(&store), published);
    assert_eq!(repair_lines(&store), journaled);
    assert_eq!(generation(&store, april), 3);
    assert_eq!(served_bars(&store, april, "NIFTY"), expected);
    std::fs::remove_dir_all(store).unwrap();
}
