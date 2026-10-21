//! IA-3's import-level proofs: what a day's close records of its listing
//! (DB-A49, FM-119), the `\` separator of a CM day zip (D-4828, §8 B22), a
//! wanted stem with no extension (D-4835, §8 D23), and the tick store's day
//! table (D-4866, §8 T9 to T11). Every row is invented at run time; every
//! price is one no index printed on these days.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_archive::Archive;
use crate::gdfl_cm::{CmKind, DayListing, ListingSum, SlotTable, day_folder_name};
use crate::gdfl_fixtures::{Method, bts, csv, put, row, scratch, zip};
use crate::gdfl_tickstore::TickStore;

fn d1() -> Day {
    Day::new(2024, 4, 1).unwrap()
}

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// An invented index file: session rows every 997 seconds and one row after
/// the close, so the reader's end check holds.
fn index_csv(stem: &str, d: Day, base: u32) -> Vec<u8> {
    let mut rows: Vec<String> = (0..22_500)
        .step_by(997)
        .map(|s| row(stem, d, NINE_FIFTEEN + s, &format!("{base}.25"), 0, 0))
        .collect();
    rows.push(row(stem, d, CLOSE + 60, &format!("{base}.50"), 0, 0));
    csv(&rows)
}

/// The index day zip of `d` with `files` (their names inside the day
/// folder), joined to the folder by `sep`, its members deflated as the
/// vendor's are, beside the folder entry.
fn index_day_zip(d: Day, sep: char, files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let folder = day_folder_name(CmKind::Indices, d);
    let mut members: Vec<(String, Vec<u8>, Method)> =
        vec![(format!("{folder}/"), Vec::new(), Method::Stored)];
    for (name, bytes) in files {
        members.push((
            format!("{folder}{sep}{name}"),
            bytes.clone(),
            Method::Deflated,
        ));
    }
    let refs: Vec<(&str, &[u8], Method)> = members
        .iter()
        .map(|(n, b, m)| (n.as_str(), b.as_slice(), *m))
        .collect();
    zip(&refs)
}

/// The outer archive holding `day_zip` as the index day zip of `d`, written
/// under `root` and opened.
fn outer(root: &Path, d: Day, day_zip: &[u8]) -> Archive<std::fs::File> {
    let name = crate::gdfl_archive::day_zip_name(CmKind::Indices, d);
    let path = put(root, "cm.zip", &zip(&[(&name, day_zip, Method::Stored)]));
    Archive::open(&path).unwrap()
}

/// NIFTY 50, NIFTY BANK and two files off the swept surface.
fn four_files(d: Day) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "NIFTY 50.NSE_IDX.csv",
            index_csv("NIFTY 50.NSE_IDX", d, 11_111),
        ),
        (
            "NIFTY BANK.NSE_IDX.csv",
            index_csv("NIFTY BANK.NSE_IDX", d, 22_222),
        ),
        (
            "NIFTY IT.NSE_IDX.csv",
            index_csv("NIFTY IT.NSE_IDX", d, 33_333),
        ),
        (
            "NSETEST.NSE_IDX.csv",
            index_csv("NSETEST.NSE_IDX", d, 44_444),
        ),
    ]
}

/// Imports indices of `d` from `source` into `store`, filtered to `only`.
fn import_indices<S>(source: &S, d: Day, store: &Path, only: &[String]) -> Report
where
    S: CmSource + Sync,
    S::Locator: Sync,
{
    let run = Run {
        kind: ImportKind::Indices,
        from: d,
        to: d,
        only,
        store_root: store,
    };
    run_cm(source, &run).unwrap()
}

/// The last closing line the journal at `store` holds for `d`.
fn close_line(store: &Path, d: Day) -> String {
    let text = std::fs::read_to_string(journal_path(store)).unwrap();
    text.lines()
        .rfind(|line| {
            ["done", "incomplete", "refused"]
                .iter()
                .any(|verb| line.starts_with(&format!("{verb} indices {d} ")))
        })
        .unwrap()
        .to_owned()
}

/// DB-A49, FM-119: a day's close says how many of its entries were not
/// wanted files and what its whole listing was (its entry count and the
/// digest of every entry's name, size and CRC-32), so a skipped file is
/// counted and identified for ever, after the event log has rotated; a
/// filtered run of the same day states the same listing and skips more.
#[test]
fn the_close_line_carries_skipped_and_the_listing() {
    let root = scratch("ia3-close-listing");
    let archive = outer(&root, d1(), &index_day_zip(d1(), '/', &four_files(d1())));
    let listing = archive.day(CmKind::Indices, d1()).unwrap().unwrap();
    let sum = ListingSum::of(listing.entries());
    assert_eq!(sum.entries, 5);
    let whole = root.join("S");
    let report = import_indices(&archive, d1(), &whole, &[]);
    assert_eq!(
        (report.files, report.files_skipped),
        (2, 3),
        "{:?}",
        report.failures
    );
    let line = close_line(&whole, d1());
    assert!(line.starts_with("done indices 2024-04-01 * "), "{line}");
    assert!(
        line.contains(&format!(" skipped=3 entries=5 listing={}", sum.hex())),
        "{line}"
    );
    let filtered = root.join("F");
    let only = ["NIFTY".to_owned()];
    let report = import_indices(&archive, d1(), &filtered, &only);
    assert_eq!((report.files, report.files_skipped), (1, 4));
    let line = close_line(&filtered, d1());
    assert!(
        line.contains(&format!(" skipped=4 entries=5 listing={}", sum.hex())),
        "{line}"
    );
    // The journal still loads, and the day reads as done: the new fields
    // move no field a reader knew.
    let journal = Journal::load(journal_path(&whole)).unwrap();
    assert!(journal.done.contains("indices 2024-04-01 *"));
}

/// A day whose listing is refused closes `refused`, which carries no counts
/// and no listing, and so does a day the source does not hold: the fields
/// never claim a listing nobody read.
#[test]
fn a_day_with_no_listing_claims_none() {
    let root = scratch("ia3-no-listing");
    let ts = root.join("ts");
    let path = TickStore::new(&ts).day_path(CmKind::Indices, d1());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"not a day file").unwrap();
    let store = root.join("S");
    let _report = import_indices(&TickStore::new(&ts), d1(), &store, &[]);
    let line = close_line(&store, d1());
    assert!(line.starts_with("refused indices 2024-04-01 * "), "{line}");
    assert!(line.contains("reason=tick_store_malformed"), "{line}");
    assert!(
        !line.contains("listing=") && !line.contains("skipped="),
        "{line}"
    );
    let next = Day::new(2024, 4, 2).unwrap();
    let _report = import_indices(&TickStore::new(&ts), next, &store, &[]);
    let line = close_line(&store, next);
    assert!(line.starts_with("refused indices 2024-04-02 "), "{line}");
    assert!(!line.contains("listing="), "{line}");
}

/// B22 (D-4828): a CM day zip whose entries use `\` reads exactly as the
/// same day zip with `/`, filtered and not: the same files, rows, seconds
/// and skips; the listing counts its backslash names.
#[test]
fn a_cm_day_zip_with_backslash_paths_reads_like_forward_slashes() {
    let root = scratch("ia3-backslash");
    let forward = outer(
        &root.join("f"),
        d1(),
        &index_day_zip(d1(), '/', &four_files(d1())),
    );
    let back = outer(
        &root.join("b"),
        d1(),
        &index_day_zip(d1(), '\\', &four_files(d1())),
    );
    let listed = back.day(CmKind::Indices, d1()).unwrap().unwrap();
    assert_eq!(listed.backslash(), 4);
    assert_eq!(
        forward
            .day(CmKind::Indices, d1())
            .unwrap()
            .unwrap()
            .backslash(),
        0
    );
    for only in [Vec::new(), vec!["NIFTY".to_owned()]] {
        let tag = only.len();
        let a = import_indices(&forward, d1(), &root.join(format!("SF{tag}")), &only);
        let b = import_indices(&back, d1(), &root.join(format!("SB{tag}")), &only);
        assert!(a.files > 0, "{:?}", a.failures);
        assert_eq!(
            (
                a.files,
                a.rows,
                a.seconds,
                a.files_skipped,
                a.failures.len()
            ),
            (
                b.files,
                b.rows,
                b.seconds,
                b.files_skipped,
                b.failures.len()
            ),
            "only={only:?}"
        );
    }
}

/// D23 (D-4835, F29): a wanted stem with no extension at all is refused by
/// name, never skipped: the day closes done with that refusal and the
/// other index read. Beside a `.csv` of the same stem, in either order, it
/// is refused once. A run filtered to another symbol skips it.
#[test]
fn a_wanted_stem_with_no_extension_is_refused_by_name() {
    let root = scratch("ia3-bare");
    let bank = (
        "NIFTY BANK.NSE_IDX.csv",
        index_csv("NIFTY BANK.NSE_IDX", d1(), 22_222),
    );
    let bare = (
        "NIFTY 50.NSE_IDX",
        index_csv("NIFTY 50.NSE_IDX", d1(), 11_111),
    );
    let csv_twin = ("NIFTY 50.NSE_IDX.csv", bare.1.clone());
    for (tag, files) in [
        ("alone", vec![bare.clone(), bank.clone()]),
        ("first", vec![bare.clone(), csv_twin.clone(), bank.clone()]),
        ("last", vec![csv_twin.clone(), bare.clone(), bank.clone()]),
    ] {
        let archive = outer(&root.join(tag), d1(), &index_day_zip(d1(), '/', &files));
        let store = root.join(tag).join("S");
        let report = import_indices(&archive, d1(), &store, &[]);
        assert_eq!(report.files, 1, "{tag}: {:?}", report.failures);
        assert_eq!(report.failures.len(), 1, "{tag}: {:?}", report.failures);
        assert!(report.failures[0].why.contains("no extension"), "{tag}");
        let line = close_line(&store, d1());
        assert!(
            line.contains("vendor_refused=1 reason=extension_missing"),
            "{tag}: {line}"
        );
        let filtered = root.join(tag).join("B");
        let report = import_indices(&archive, d1(), &filtered, &["BANKNIFTY".to_owned()]);
        assert_eq!((report.files, report.failures.len()), (1, 0), "{tag}");
    }
    // The table answers the same for a share stem, in either order.
    let mut table = SlotTable::new(CmKind::Stocks);
    table.file("RELIANCE.NSE.csv", 0_usize);
    table.file("RELIANCE.NSE", 1);
    assert_eq!(
        table.get("RELIANCE.NSE").err(),
        Some(CmRefusal::ExtensionMissing {
            stem: "RELIANCE.NSE".to_owned()
        })
    );
    let mut listing: DayListing<()> = DayListing::new(CmKind::Stocks, d1());
    let folder = listing.folder().to_owned();
    listing.push(&format!("{folder}/SBIN.NSE"), 1, 0, ());
    listing.push(&format!("{folder}/SBIN.NSE.Bak"), 1, 0, ());
    assert_eq!(
        listing.locate("SBIN.NSE").err(),
        Some(CmRefusal::ExtensionMissing {
            stem: "SBIN.NSE".to_owned()
        })
    );
}

/// The tick store's day file of `d`, at `rel` under `ts`.
fn day_file_at(ts: &Path, rel: &str, d: Day) {
    let folder = day_folder_name(CmKind::Indices, d);
    let name = format!("{folder}/NIFTY 50.NSE_IDX.csv");
    let body = index_csv("NIFTY 50.NSE_IDX", d, 11_111);
    put(ts, rel, &bts(&[(&name, &body)]));
}

/// T10 (D-4866): a `.bts` outside its own folder refuses its day by name,
/// where it was found and where it belongs, and never closes it; the day is
/// not `day_not_held`.
#[test]
fn a_misfiled_tick_store_day_is_named_where_it_was_found() {
    let root = scratch("ia3-ts-misfiled");
    let ts = root.join("ts");
    let found = "cm/INDICES/2024/MAY_2024/GFDLCM_INDICES_TICK_01042024.bts";
    day_file_at(&ts, found, d1());
    let store = TickStore::new(&ts);
    assert_eq!(
        store.day(CmKind::Indices, d1()).err(),
        Some(CmRefusal::DayFileMisfiled {
            found: found.to_owned(),
            expected: "cm/INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.bts".to_owned(),
        })
    );
    let out = root.join("S");
    let _report = import_indices(&store, d1(), &out, &[]);
    let line = close_line(&out, d1());
    assert!(
        line.starts_with("refused indices 2024-04-01 * ")
            && line.contains("stage=vendor reason=day_file_misfiled"),
        "{line}"
    );
    // A copy at its own place beside it is no escape: the day is refused
    // until the stray copy is moved, as the archives refuse a second copy.
    let both = root.join("both");
    day_file_at(&both, found, d1());
    let own = TickStore::new(&both).day_path(CmKind::Indices, d1());
    day_file_at(
        &both,
        own.strip_prefix(&both).unwrap().to_str().unwrap(),
        d1(),
    );
    let both = TickStore::new(&both);
    assert!(matches!(
        both.day(CmKind::Indices, d1()),
        Err(CmRefusal::DayFileMisfiled { .. })
    ));
    assert_eq!((both.table().held, both.table().misfiled_count), (1, 1));
}

/// Makes a pipe at `path`: a file any open would wait on for ever.
fn pipe(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo {}", path.display());
}

/// T9 and T11 (D-4866): the writer's own files, a `*.bts.tmp` and anything
/// else beside the day files are stray entries, counted and never opened
/// (each is a pipe here, so an open would hang the test); a day the writer
/// publishes after the walk is read whole.
#[test]
fn the_tick_stores_writer_files_are_never_opened() {
    let root = scratch("ia3-ts-writer");
    let ts = root.join("ts");
    let april = "cm/INDICES/2024/APR_2024";
    day_file_at(
        &ts,
        &format!("{april}/GFDLCM_INDICES_TICK_01042024.bts"),
        d1(),
    );
    pipe(&ts.join(format!("{april}/GFDLCM_INDICES_TICK_02042024.bts.tmp")));
    pipe(&ts.join("cm/INDICES/.lock"));
    pipe(&ts.join("cm/ingest-log.tsv"));
    let store = TickStore::new(&ts);
    assert!(store.day(CmKind::Indices, d1()).unwrap().is_some());
    let table = store.table();
    assert_eq!((table.held, table.stray, table.misfiled_count), (1, 3, 0));
    assert_eq!(table.fault, None);
    // Published by rename after the walk: read at its own place, whole.
    let next = Day::new(2024, 4, 2).unwrap();
    assert!(store.day(CmKind::Indices, next).unwrap().is_none());
    day_file_at(
        &ts,
        &format!("{april}/GFDLCM_INDICES_TICK_02042024.bts"),
        next,
    );
    let listing = store.day(CmKind::Indices, next).unwrap().unwrap();
    assert_eq!(listing.entries().len(), 1);
}

/// The p50, p99 and max of `samples`, in nanoseconds, sorted in place.
fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |q: usize| samples.get((samples.len() * q / 100).min(samples.len().saturating_sub(1)));
    (
        at(50).copied().unwrap_or(0),
        at(99).copied().unwrap_or(0),
        samples.last().copied().unwrap_or(0),
    )
}

/// The costs IA-3's lookups pay, measured, not argued (`docs/06-limits.md`
/// L-IA-8, L-IA-59, L-IA-61): each walk once, cold, then n day lookups of
/// the walked table, each sample one call, sequential. Over the operator's
/// real sources where `BRUTEX_GDFL_TICKSTORE`, `BRUTEX_GDFL_CM_ARCHIVE` and
/// `BRUTEX_GDFL_DAYZIPS` name them, read only. Run with `--ignored
/// --nocapture` in release.
#[test]
#[ignore = "a timing run over the operator's real sources, named by variables"]
fn ia3_lookup_costs() {
    let days: Vec<Day> = (0..2_600)
        .filter_map(|n| Day::from_days(Day::new(2018, 9, 3).unwrap().days_from_epoch() + n).ok())
        .collect();
    if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
        tickstore_costs(Path::new(&root), &days);
    }
    if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
        archive_costs(Path::new(&path), &days);
    }
    if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
        dayzips_costs(Path::new(&root), &days);
    }
}

/// The tick store's walks, its misfiled probe, its day listings and their
/// digests (L-IA-8, L-IA-59).
fn tickstore_costs(root: &Path, days: &[Day]) {
    let store = TickStore::new(root);
    let cold = std::time::Instant::now();
    let table = store.table();
    println!(
        "tickstore cm walk: {} ms, held={} misfiled={} stray={} fault={:?} first strays={:?}",
        cold.elapsed().as_millis(),
        table.held,
        table.misfiled_count,
        table.stray,
        table.fault,
        table.stray_named.iter().take(4).collect::<Vec<_>>()
    );
    let mut probe: Vec<u128> = Vec::new();
    for day in days.iter().cycle().take(20_000) {
        let at = std::time::Instant::now();
        let _ = std::hint::black_box(table.misfiled(0, *day));
        probe.push(at.elapsed().as_nanos());
    }
    let n = probe.len();
    let (p50, p99, max) = quantiles(&mut probe);
    println!("tickstore misfiled probe: n={n} p50={p50} ns p99={p99} ns max={max} ns");
    let mut listing: Vec<u128> = Vec::new();
    let mut sums: Vec<u128> = Vec::new();
    for day in days.iter().take(400) {
        let at = std::time::Instant::now();
        let got = store.day(CmKind::Indices, *day);
        listing.push(at.elapsed().as_nanos());
        if let Ok(Some(listed)) = got {
            let at = std::time::Instant::now();
            let _ = std::hint::black_box(ListingSum::of(listed.entries()));
            sums.push(at.elapsed().as_nanos());
        }
    }
    let n = listing.len();
    let (p50, p99, max) = quantiles(&mut listing);
    println!("tickstore day listing (open + index): n={n} p50={p50} ns p99={p99} ns max={max} ns");
    let n = sums.len();
    let (p50, p99, max) = quantiles(&mut sums);
    println!("indices listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
    let nfo = crate::gdfl_nfo::NfoTickStore::new(root);
    let cold = std::time::Instant::now();
    let table = nfo.table();
    println!(
        "tickstore options walk: {} ms, held={} misfiled={} stray={}",
        cold.elapsed().as_millis(),
        table.held,
        table.misfiled_count,
        table.stray
    );
    let mut sums: Vec<u128> = Vec::new();
    for day in days.iter().rev().take(60) {
        if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
            let at = std::time::Instant::now();
            let _ = std::hint::black_box(ListingSum::of(listed.entries()));
            sums.push(at.elapsed().as_nanos());
        }
    }
    let n = sums.len();
    let (p50, p99, max) = quantiles(&mut sums);
    println!("options listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
}

/// The archive's directory read and its day listings (L-IA-61).
fn archive_costs(path: &Path, days: &[Day]) {
    let cold = std::time::Instant::now();
    let archive = Archive::open(path).unwrap();
    println!(
        "cm archive open (directory read once): {} ms, census={:?}",
        cold.elapsed().as_millis(),
        archive.census()
    );
    let mut lookups: Vec<u128> = Vec::new();
    for day in days.iter().take(400) {
        let at = std::time::Instant::now();
        let _ = std::hint::black_box(archive.day(CmKind::Indices, *day));
        lookups.push(at.elapsed().as_nanos());
    }
    let n = lookups.len();
    let (p50, p99, max) = quantiles(&mut lookups);
    println!(
        "cm archive day listing (index + day zip directory): n={n} p50={p50} ns p99={p99} ns max={max} ns"
    );
}

/// The loose root's walk and its day lookups (L-IA-8).
fn dayzips_costs(root: &Path, days: &[Day]) {
    let cold = std::time::Instant::now();
    let loose = crate::gdfl_dayzips::DayZips::open(root).unwrap();
    println!(
        "dayzips walk: {} us, census={:?}",
        cold.elapsed().as_micros(),
        loose.census()
    );
    let mut lookups: Vec<u128> = Vec::new();
    for day in days.iter().rev().take(400) {
        let at = std::time::Instant::now();
        let _ = std::hint::black_box(CmSource::day(&loose, CmKind::Indices, *day));
        lookups.push(at.elapsed().as_nanos());
    }
    let n = lookups.len();
    let (p50, p99, max) = quantiles(&mut lookups);
    println!(
        "dayzips day lookup (probe, and open + directory where held): n={n} p50={p50} ns p99={p99} ns max={max} ns"
    );
}

/// The last closing line of `kind` on `d` in the journal at `store`.
fn close_of(store: &Path, kind: &str, d: Day) -> String {
    let text = std::fs::read_to_string(journal_path(store)).unwrap_or_default();
    text.lines()
        .rfind(|line| {
            ["done", "incomplete", "refused"]
                .iter()
                .any(|verb| line.starts_with(&format!("{verb} {kind} {d} ")))
        })
        .unwrap_or_default()
        .to_owned()
}

/// A run of `kind` over the one day `d`, filtered to `only`, into `at`.
fn one_day<'a>(kind: ImportKind, d: Day, only: &'a [String], at: &'a Path) -> Run<'a> {
    Run {
        kind,
        from: d,
        to: d,
        only,
        store_root: at,
    }
}

/// The `listing=` field of a close line.
fn listing_of(line: &str) -> Option<&str> {
    line.split(' ')
        .find_map(|field| field.strip_prefix("listing="))
}

/// NIFTY FIRST, on the operator's real sources (read only; every write goes
/// to a fresh scratch store): one NIFTY spot day, one special session, one
/// NIFTY options day, one BANKNIFTY day, one F&O share and one loose day,
/// each imported from two sources that hold the same vendor day, and each
/// close's listing digest compared. Two copies of one day zip agree on every
/// entry's name, size and CRC-32, so their digests must be equal; a
/// difference is a finding. Run with `--ignored --nocapture`.
#[test]
#[ignore = "imports real days from the operator's sources into scratch stores"]
fn real_days_close_with_the_same_listing_from_two_sources() {
    let var = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
    let (Some(ts), Some(cm), Some(nfo)) = (
        var("BRUTEX_GDFL_TICKSTORE"),
        var("BRUTEX_GDFL_CM_ARCHIVE"),
        var("BRUTEX_GDFL_NFO_ARCHIVES"),
    ) else {
        return;
    };
    let archive = Archive::open(&cm).unwrap();
    let store = TickStore::new(&ts);
    let root = scratch("ia3-real");
    let cm_case = |tag: &str, kind: ImportKind, d: Day, only: &[String]| {
        let a = root.join(format!("{tag}-zip"));
        let b = root.join(format!("{tag}-ts"));
        let zip_report = run_cm(&archive, &one_day(kind, d, only, &a)).unwrap();
        let ts_report = run_cm(&store, &one_day(kind, d, only, &b)).unwrap();
        let (zip_line, store_line) = (
            close_of(&a, kind.as_str(), d),
            close_of(&b, kind.as_str(), d),
        );
        println!("{tag} zip: {zip_line}");
        println!("{tag} tickstore: {store_line}");
        assert!(listing_of(&zip_line).is_some(), "{zip_line}");
        assert_eq!(listing_of(&zip_line), listing_of(&store_line), "{tag}");
        assert_eq!(
            (
                zip_report.files,
                zip_report.seconds,
                zip_report.files_skipped
            ),
            (ts_report.files, ts_report.seconds, ts_report.files_skipped),
            "{tag}"
        );
    };
    let nifty = ["NIFTY".to_owned()];
    cm_case(
        "nifty-spot",
        ImportKind::Indices,
        Day::new(2024, 4, 1).unwrap(),
        &nifty,
    );
    cm_case(
        "muhurat",
        ImportKind::Indices,
        Day::new(2024, 11, 1).unwrap(),
        &nifty,
    );
    cm_case(
        "banknifty",
        ImportKind::Indices,
        Day::new(2020, 3, 23).unwrap(),
        &["BANKNIFTY".to_owned()],
    );
    cm_case(
        "reliance",
        ImportKind::Stocks,
        Day::new(2024, 4, 1).unwrap(),
        &["RELIANCE".to_owned()],
    );
    let d = Day::new(2024, 4, 4).unwrap();
    let a = root.join("nifty-options-zip");
    let b = root.join("nifty-options-ts");
    let options = ImportKind::Options;
    let zips = run_nfo(
        &crate::gdfl_nfo::NfoZips::new(&nfo),
        &one_day(options, d, &nifty, &a),
    )
    .unwrap();
    let tick = run_nfo(
        &crate::gdfl_nfo::NfoTickStore::new(&ts),
        &one_day(options, d, &nifty, &b),
    )
    .unwrap();
    let (zip_line, store_line) = (close_of(&a, "options", d), close_of(&b, "options", d));
    println!("nifty-options zip: {zip_line}");
    println!("nifty-options tickstore: {store_line}");
    assert!(listing_of(&zip_line).is_some(), "{zip_line}");
    assert_eq!(listing_of(&zip_line), listing_of(&store_line));
    assert_eq!((zips.files, zips.seconds), (tick.files, tick.seconds));
    if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
        let loose = crate::gdfl_dayzips::DayZips::open(&loose).unwrap();
        let d = Day::new(2026, 9, 25).unwrap();
        let a = root.join("loose-nifty");
        let b = root.join("loose-nifty-ts");
        let indices = ImportKind::Indices;
        let from_loose = run_cm(&loose, &one_day(indices, d, &nifty, &a)).unwrap();
        let from_ts = run_cm(&store, &one_day(indices, d, &nifty, &b)).unwrap();
        let (zip_line, store_line) = (close_of(&a, "indices", d), close_of(&b, "indices", d));
        println!("loose 2026-09-25: {zip_line}");
        println!("tickstore 2026-09-25: {store_line}");
        assert_eq!(listing_of(&zip_line), listing_of(&store_line));
        assert_eq!(
            (from_loose.files, from_loose.seconds),
            (from_ts.files, from_ts.seconds)
        );
    }
    println!("scratch stores under {}", root.display());
}

/// The second day the container-fault proofs import beside the faulty one.
fn d2() -> Day {
    Day::new(2024, 4, 2).unwrap()
}

/// The outer archive of `members` (each a name, its bytes and how the outer
/// zip stores it), written under `root` and opened.
fn outer_of(root: &Path, members: &[(String, Vec<u8>, Method)]) -> Archive<std::fs::File> {
    let refs: Vec<(&str, &[u8], Method)> = members
        .iter()
        .map(|(n, b, m)| (n.as_str(), b.as_slice(), *m))
        .collect();
    Archive::open(&put(root, "cm.zip", &zip(&refs))).unwrap()
}

/// The good index day zip of `d`, stored at its own place.
fn good_day(d: Day) -> (String, Vec<u8>, Method) {
    (
        crate::gdfl_archive::day_zip_name(CmKind::Indices, d),
        index_day_zip(d, '/', &four_files(d)),
        Method::Stored,
    )
}

/// What one container fault leaves, imported twice over `d1..d2` from
/// `archive` (IA-GR-27): `d1` is refused with `code` and never closed, so the
/// second run reads it again and fails it again, `d2` closes done with its
/// two swept files, and every failure names `named`.
fn faulty_day_stays_open(archive: &Archive<std::fs::File>, store: &Path, code: &str, named: &str) {
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: store,
    };
    for pass in 0..2 {
        let report = run_cm(archive, &run).unwrap();
        assert_eq!(
            report.failures.len(),
            1,
            "pass {pass}: {:?}",
            report.failures
        );
        assert!(
            report.failures[0].why.contains(named),
            "pass {pass}: {:?}",
            report.failures
        );
        // The first pass reads both days; the second only the open one.
        let want_files = if pass == 0 { 2 } else { 0 };
        assert_eq!(report.files, want_files, "pass {pass}");
    }
    let line = close_line(store, d1());
    assert!(line.starts_with("refused indices 2024-04-01 "), "{line}");
    assert!(line.contains(&format!("reason={code}")), "{line}");
    assert!(!line.contains("day_not_held"), "{line}");
    let journal = Journal::load(journal_path(store)).unwrap();
    assert!(!journal.done.contains("indices 2024-04-01 *"));
    assert!(journal.done.contains("indices 2024-04-02 *"));
}

/// FM-023 (DATA-006, CONC-067, design §8 C3): a day zip in the wrong month
/// folder of the outer archive is named where it was found and where its
/// name files it, never counted silently into `ignored` and never read as
/// `day_not_held`; its day stays open and the next day imports.
#[test]
fn a_day_zip_in_the_wrong_month_folder_is_named_not_ignored() {
    let root = scratch("ia3-fm023");
    let expected = crate::gdfl_archive::day_zip_name(CmKind::Indices, d1());
    let found = expected.replace("/APR_2024/", "/MAY_2024/");
    assert_ne!(found, expected);
    let archive = outer_of(
        &root,
        &[
            (
                found.clone(),
                index_day_zip(d1(), '/', &four_files(d1())),
                Method::Stored,
            ),
            good_day(d2()),
        ],
    );
    assert_eq!(archive.census().misfiled, 1);
    assert_eq!(archive.census().misfiled_named, vec![found.clone()]);
    assert_eq!(
        archive.day(CmKind::Indices, d1()).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: found.clone(),
            expected: expected.clone()
        })
    );
    faulty_day_stays_open(&archive, &root.join("S"), "day_zip_misfiled", &found);
    let journal = Journal::load(journal_path(&root.join("S"))).unwrap();
    assert!(!journal.done.contains("indices 2024-04-01 *"));
}

/// FM-044 (DATA-008, design §8 B19): two day zips for one tree and day
/// refuse that day only, wherever the second lies; the archive opens and
/// the other day lists and imports.
#[test]
fn two_members_for_one_day_refuse_that_day_only() {
    let root = scratch("ia3-fm044");
    let own = good_day(d1());
    let copy = (
        own.0.replace(".zip", " (1).zip"),
        own.1.clone(),
        Method::Stored,
    );
    let archive = outer_of(&root, &[own, copy, good_day(d2())]);
    assert_eq!(
        archive.day(CmKind::Indices, d1()).err(),
        Some(CmRefusal::ArchiveDuplicateDay { day: d1() })
    );
    assert!(archive.day(CmKind::Indices, d2()).unwrap().is_some());
    faulty_day_stays_open(
        &archive,
        &root.join("S"),
        "archive_duplicate_day",
        "2024-04-01",
    );
}

/// FM-040 (DATA-058, design §8 B8): a day zip the outer archive stores
/// deflated is refused for its own day by name, never for the archive, and
/// its day stays open while the next day imports.
#[test]
fn a_deflated_day_zip_member_is_read_or_refused_only_for_its_day() {
    let root = scratch("ia3-fm040");
    let (name, bytes, _) = good_day(d1());
    let archive = outer_of(
        &root,
        &[(name.clone(), bytes, Method::Deflated), good_day(d2())],
    );
    assert_eq!(
        archive.day(CmKind::Indices, d1()).err(),
        Some(CmRefusal::ArchiveMemberCompressed {
            member: name.clone()
        })
    );
    faulty_day_stays_open(
        &archive,
        &root.join("S"),
        "archive_member_compressed",
        &name,
    );
}

/// HUMAN-010 (FM-040): the compressed day zip is refused from the central
/// directory alone, so its bytes are never inflated, let alone opened as a
/// zip: overwriting every byte of its data with garbage changes nothing.
#[test]
fn a_deflated_day_zip_is_refused_and_never_recursed() {
    let root = scratch("ia3-human010");
    let (name, bytes, _) = good_day(d1());
    let mut outer_bytes = zip(&[(name.as_str(), bytes.as_slice(), Method::Deflated)]);
    // The member's data runs from the end of its local header (30 bytes and
    // its name; the fixture writes no extra field) to the central directory.
    let data = 30 + name.len();
    let central = outer_central(&outer_bytes);
    for byte in &mut outer_bytes[data..central] {
        *byte = 0xA5;
    }
    let path = put(&root, "cm.zip", &outer_bytes);
    let archive = Archive::open(&path).unwrap();
    assert_eq!(
        archive.day(CmKind::Indices, d1()).err(),
        Some(CmRefusal::ArchiveMemberCompressed { member: name })
    );
}

/// FM-026 (HUMAN-016, design §8 B12): a day zip whose general-purpose flag
/// sets bit 0 is refused `archive_member_encrypted` by name before any byte
/// is read, never as a corrupt member or a CRC mismatch; its day stays open
/// and the next day imports.
#[test]
fn an_encrypted_member_is_refused_as_encrypted_not_corrupt() {
    let root = scratch("ia3-fm026");
    let (name, bytes, method) = good_day(d1());
    let (next, next_bytes, _) = good_day(d2());
    let mut outer_bytes = zip(&[
        (name.as_str(), bytes.as_slice(), method),
        (next.as_str(), next_bytes.as_slice(), Method::Stored),
    ]);
    // Bit 0 of the first member's flag, in its local header (offset 6) and
    // its central record (offset 8).
    outer_bytes[6] |= 1;
    let central = outer_central(&outer_bytes);
    outer_bytes[central + 8] |= 1;
    let archive = Archive::open(&put(&root, "cm.zip", &outer_bytes)).unwrap();
    let why = archive.day(CmKind::Indices, d1()).unwrap_err();
    assert_eq!(
        why,
        CmRefusal::ArchiveMemberEncrypted {
            member: name.clone()
        }
    );
    assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
    faulty_day_stays_open(&archive, &root.join("S"), "archive_member_encrypted", &name);
}

/// FM-119 (DATA-065): the close line carries, beside the late rows left
/// unresolved, how many entries of the day were skipped and what the whole
/// listing was, so a zero-file day still leaves a lasting trace of what it
/// held: a day zip holding only files off the swept surface closes with
/// `files=0 skipped=<every file>` and the listing's digest.
#[test]
fn the_close_line_carries_skipped_and_unresolved() {
    let root = scratch("ia3-fm119");
    let off_surface: Vec<(&str, Vec<u8>)> = four_files(d1()).into_iter().skip(2).collect();
    let archive = outer(&root, d1(), &index_day_zip(d1(), '/', &off_surface));
    let sum = ListingSum::of(
        archive
            .day(CmKind::Indices, d1())
            .unwrap()
            .unwrap()
            .entries(),
    );
    let store = root.join("S");
    let report = import_indices(&archive, d1(), &store, &[]);
    assert_eq!((report.files, report.files_skipped), (0, 3));
    let line = close_line(&store, d1());
    assert!(line.contains(" files=0 "), "{line}");
    assert!(line.contains(" late_unresolved=0 "), "{line}");
    assert!(
        line.contains(&format!(" skipped=3 entries=3 listing={}", sum.hex())),
        "{line}"
    );
}

/// RA-305 (design §8 C15, the import half): a leap day's day zip is filed by
/// the same day table as any other and imports done, and a day-zip name for
/// 29 February of a plain year is named in the archive's census as invalid,
/// never filed, so it can neither size the table nor stand for 1 March.
#[test]
fn the_leap_day_imports_and_29_feb_of_a_plain_year_is_named() {
    let root = scratch("ia3-leap");
    let leap = Day::new(2024, 2, 29).unwrap();
    let plain = "INDICES/2023/FEB_2023/GFDLCM_INDICES_TICK_29022023.zip".to_owned();
    let archive = outer_of(
        &root,
        &[
            good_day(leap),
            (
                plain.clone(),
                index_day_zip(leap, '/', &four_files(leap)),
                Method::Stored,
            ),
        ],
    );
    assert_eq!(archive.census().invalid, 1);
    assert_eq!(archive.census().invalid_named, vec![plain]);
    let march = Day::new(2023, 3, 1).unwrap();
    assert!(archive.day(CmKind::Indices, march).unwrap().is_none());
    let store = root.join("S");
    let report = import_indices(&archive, leap, &store, &[]);
    assert_eq!(
        (report.files, report.files_skipped, report.failures.len()),
        (2, 3, 0),
        "{:?}",
        report.failures
    );
    let line = close_line(&store, leap);
    assert!(line.starts_with("done indices 2024-02-29 * "), "{line}");
}

/// Where the outer zip's own central directory starts, read from its end
/// record (the fixture writes no comment, so the record is its last 22
/// bytes). A stored day zip carries its own directory inside the outer
/// zip's data, so a search for the first central signature would find the
/// inner one.
fn outer_central(bytes: &[u8]) -> usize {
    let end = bytes.len() - 22;
    assert_eq!(bytes[end..end + 4], [0x50, 0x4b, 0x05, 0x06]);
    let at = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap());
    let at = usize::try_from(at).unwrap();
    assert_eq!(bytes[at..at + 4], [0x50, 0x4b, 0x01, 0x02]);
    at
}

/// One invented options file of `ticker` on `d`: a row every 2,003 seconds
/// of the session, at a price no NIFTY option printed.
fn option_csv(ticker: &str, d: Day) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let rows: Vec<String> = (0..22_500)
        .step_by(2_003)
        .map(|s| {
            row(
                &stem,
                d,
                NINE_FIFTEEN + s,
                "101.25",
                u64::from(s % 3) * 25,
                1_000,
            )
        })
        .collect();
    csv(&rows)
}

/// The options day zip of `d`: one NIFTY call, its member deflated as the
/// vendor's are, under the day folder with the vendor's `\` separator.
fn options_day_zip(d: Day) -> Vec<u8> {
    let ticker = "NIFTY04APR2422000CE";
    let name = format!(
        "{}\\Options\\{ticker}.NFO.csv",
        crate::gdfl_nfo::day_folder_name(d)
    );
    zip(&[(&name, &option_csv(ticker, d), Method::Deflated)])
}

/// The members of one invented zip: each a name, its bytes and how it is
/// stored.
type Members = Vec<(String, Vec<u8>, Method)>;

/// The year zips `zips` (each a file name and its members) written under
/// `root/<tag>`, whose directory is returned.
fn year_zips(root: &Path, tag: &str, zips: &[(&str, Members)]) -> PathBuf {
    let dir = root.join(tag);
    for (name, members) in zips {
        let refs: Vec<(&str, &[u8], Method)> = members
            .iter()
            .map(|(n, b, m)| (n.as_str(), b.as_slice(), *m))
            .collect();
        put(&dir, name, &zip(&refs));
    }
    dir
}

/// What one options container fault leaves, imported twice over `d1..d2`
/// from `source` (IA-GR-27, the options half): `d1` is refused with `code`,
/// named by `named`, and never closed, so the second run reads it again and
/// fails it again; `d2` closes done with its one file.
fn faulty_options_day_stays_open<S>(source: &S, store: &Path, code: &str, named: &str)
where
    S: crate::gdfl_nfo::NfoSource + Sync,
    S::Locator: Sync,
{
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: store,
    };
    for pass in 0..2 {
        let report = run_nfo(source, &run).unwrap();
        assert_eq!(
            report.failures.len(),
            1,
            "{code} pass {pass}: {:?}",
            report.failures
        );
        assert!(
            report.failures[0].why.contains(named),
            "{code} pass {pass}: {:?}",
            report.failures
        );
        let want_files = usize::from(pass == 0);
        assert_eq!(report.files, want_files, "{code} pass {pass}");
    }
    let line = close_of(store, "options", d1());
    assert!(
        line.starts_with("refused options 2024-04-01 "),
        "{code}: {line}"
    );
    assert!(line.contains(&format!("reason={code}")), "{code}: {line}");
    assert!(!line.contains("day_not_held"), "{code}: {line}");
    let done = close_of(store, "options", d2());
    assert!(
        done.starts_with("done options 2024-04-02 "),
        "{code}: {done}"
    );
    let journal = Journal::load(journal_path(store)).unwrap();
    assert!(!journal.done.contains("options 2024-04-01 *"), "{code}");
    assert!(journal.done.contains("options 2024-04-02 *"), "{code}");
}

/// IA-GR-27, the options half (IA-3-OPT, FM-043): every container refusal an
/// options source raises reaches its `cm_cause` arm through
/// `NfoRefusal::Source`, unchanged, and none of them closes a day: imported
/// twice, the faulty day is refused by its own code both times and the next
/// day closes done. Proved from the sources themselves: a day zip in the
/// next year's zip, two copies of one day across two year zips, a day zip
/// the year zip compresses, an encrypted day zip, and a tick-store day file
/// under another month's folder.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixture per options container fault, each imported twice"
)]
fn every_options_source_refusal_reaches_its_cm_arm() {
    use crate::gdfl_import::{cm_cause, nfo_cause};
    use crate::gdfl_nfo::{NfoRefusal, NfoTickStore, NfoZips};
    // The arms themselves, one value of each container and source refusal.
    let arms = [
        CmRefusal::ArchiveDuplicateDay { day: d1() },
        CmRefusal::ArchiveMemberCompressed { member: "m".into() },
        CmRefusal::ArchiveMethodUnknown {
            member: "m".into(),
            method: 12,
        },
        CmRefusal::ArchiveMemberEncrypted { member: "m".into() },
        CmRefusal::ArchivePrefixed { bytes: 7 },
        CmRefusal::DayZipMisfiled {
            found: "f".into(),
            expected: "e".into(),
        },
        CmRefusal::DayFileMisfiled {
            found: "f".into(),
            expected: "e".into(),
        },
        CmRefusal::ArchiveUnavailable {
            kind: std::io::ErrorKind::PermissionDenied,
        },
        CmRefusal::ArchiveMalformed { what: "w" },
        CmRefusal::SourceLengthMismatch { file: 1, member: 2 },
        CmRefusal::SourceCrcMismatch { file: 1, member: 2 },
        CmRefusal::ArchiveMemberCorrupt { member: "m".into() },
        CmRefusal::TickStoreMalformed { what: "w" },
        CmRefusal::TickStoreUnavailable {
            kind: std::io::ErrorKind::PermissionDenied,
        },
    ];
    for why in arms {
        let cause = nfo_cause(&NfoRefusal::Source(why.clone()));
        assert_eq!(cause, cm_cause(&why), "{why:?}");
        assert!(!cause.closes, "{why:?} closes a day");
    }

    let root = scratch("ia3-opt-gr27");
    let own = |d: Day, method: Method| {
        (
            format!("APR_2024/{}.zip", crate::gdfl_nfo::day_folder_name(d)),
            options_day_zip(d),
            method,
        )
    };
    // A day zip in the next year's zip.
    let misfiled = year_zips(
        &root,
        "misfiled",
        &[
            ("2024.zip", vec![own(d2(), Method::Stored)]),
            ("2025.zip", vec![own(d1(), Method::Stored)]),
        ],
    );
    faulty_options_day_stays_open(
        &NfoZips::new(&misfiled),
        &root.join("S-misfiled"),
        "day_zip_misfiled",
        "2025.zip:APR_2024/GFDLNFO_TICK_01042024.zip",
    );
    // Two copies of one day, one in each of two year zips.
    let twice = year_zips(
        &root,
        "twice",
        &[
            (
                "2024.zip",
                vec![own(d1(), Method::Stored), own(d2(), Method::Stored)],
            ),
            ("2025.zip", vec![own(d1(), Method::Stored)]),
        ],
    );
    faulty_options_day_stays_open(
        &NfoZips::new(&twice),
        &root.join("S-twice"),
        "archive_duplicate_day",
        "2024-04-01",
    );
    // A day zip the year zip compresses.
    let packed = year_zips(
        &root,
        "packed",
        &[(
            "2024.zip",
            vec![own(d1(), Method::Deflated), own(d2(), Method::Stored)],
        )],
    );
    faulty_options_day_stays_open(
        &NfoZips::new(&packed),
        &root.join("S-packed"),
        "archive_member_compressed",
        "GFDLNFO_TICK_01042024.zip",
    );
    // An encrypted day zip: bit 0 of its flag in its local header and its
    // central record.
    let encrypted = year_zips(
        &root,
        "encrypted",
        &[(
            "2024.zip",
            vec![own(d1(), Method::Stored), own(d2(), Method::Stored)],
        )],
    );
    let path = encrypted.join("2024.zip");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[6] |= 1;
    let central = outer_central(&bytes);
    bytes[central + 8] |= 1;
    std::fs::write(&path, &bytes).unwrap();
    faulty_options_day_stays_open(
        &NfoZips::new(&encrypted),
        &root.join("S-encrypted"),
        "archive_member_encrypted",
        "GFDLNFO_TICK_01042024.zip",
    );
    // A tick-store day file under another month's folder.
    let ts = root.join("ts");
    let entry = |d: Day| {
        format!(
            "{}\\Options\\NIFTY04APR2422000CE.NFO.csv",
            crate::gdfl_nfo::day_folder_name(d)
        )
    };
    let (e1, e2) = (entry(d1()), entry(d2()));
    let (f1, f2) = (
        option_csv("NIFTY04APR2422000CE", d1()),
        option_csv("NIFTY04APR2422000CE", d2()),
    );
    put(
        &ts,
        "options/2024/MAY_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&[(&e1, &f1)]),
    );
    put(
        &ts,
        "options/2024/APR_2024/GFDLNFO_TICK_02042024.bts",
        &bts(&[(&e2, &f2)]),
    );
    faulty_options_day_stays_open(
        &NfoTickStore::new(&ts),
        &root.join("S-ts"),
        "day_file_misfiled",
        "MAY_2024",
    );
}
