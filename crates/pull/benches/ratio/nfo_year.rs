//! C-CHK-02 and C-CHK-03 (CC-20, FM-615, D-6801): the options year index,
//! `NfoZips::year`, which is private to `crates/pull/src/gdfl_nfo.rs` (owned
//! by PREC-6 while it is live, and edited by IA-4 and R4-22, none merged on
//! 24 Oct 2026), so it is timed through its public caller,
//! `<NfoZips as NfoSource>::day`, on invented zips in a new folder. No seam
//! was added to that file.
//!
//! C-CHK-02 is the FIRST `day()` of a run: it lists the directory once and
//! walks every `*.zip` in it, building each `<yyyy>.zip`'s index from its
//! central directory (`list` calls `year` for each), then lists the asked
//! day's own zip. O(names) plus O(entries) per zip, ONCE PER RUN: the bench
//! puts one year zip in the folder, times that first call at 1x, 10x and 100x
//! the year zip's entries (30, 300 and 3,000), prints the per-run time at
//! each, and holds its cost PER ENTRY to the ceiling across each tenfold step
//! (`linear_per_unit`, the rule for an O(n) cost once per run, D-6800).
//!
//! C-CHK-03 is every LATER `day()` of the run, of any year: the index is a
//! hash hit, the misfiled notes one more probe, then that day zip's own
//! listing. Its cost must be flat at 1x, 10x and 100x the year zip, the day
//! zips kept the same size at every scale.
//!
//! A year holds at most 366 day names at their own place, so a year zip past
//! that is padded as the vendor's are: a tenth of each scale is day zips at
//! their own place, a tenth is day zips of the year before packed into this
//! year's zip (each noted as misfiled, the year boundary crossed), and the
//! rest are month folders and stray files the walk skips. The mix is the same
//! at every scale, so the cost per entry compares like with like.
//!
//! Real files only: the index keeps an open `std::fs::File` and reads its
//! directory with positional reads, so (unlike the capital-market archive)
//! there is no in-memory source, and the file open, `metadata` and reads are
//! in the figure, from the page cache. The folders are left where they are
//! for clean-up (R4-08's no-delete rule) and printed. The harness,
//! `crates/pull/benches/ratio.rs`, fails on a ratio above its ceiling and
//! prints p50, p99 and max beside each.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pull::gdfl_cm::CmRefusal;
use pull::gdfl_nfo::{NfoSource, NfoZips, day_folder_name};
use pull::session::Day;

use super::archive_from_source::{linear_per_unit, stored_zip};
use super::{cost_ps, ratio, refuse, spread};

/// Entries of the 1x year zip; 10x and 100x hold ten and a hundred times as
/// many.
const BASE_ENTRIES: u32 = 30;
/// Fresh sources timed per size for the first call; the minimum is kept.
const FIRST_TRIALS: u32 = 40;
/// The year every year zip here is named for.
const YEAR: u16 = 2025;

/// A new, empty folder under the system temp directory, printed.
fn folder(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-chk-nfo-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    if std::fs::create_dir_all(&dir).is_err() {
        refuse("a scratch folder for the year zips");
    }
    println!("  C-CHK-02 scratch (left for clean-up): {}", dir.display());
    dir
}

/// The `n`th day of [`YEAR`], from 2 January.
fn own_day(n: u32) -> Day {
    let first = Day::new(YEAR, 1, 2).unwrap_or_else(|_| refuse("a real day"));
    Day::from_days(first.days_from_epoch() + n).unwrap_or_else(|_| refuse("a day in range"))
}

/// The `n`th day back from 31 December of the year before.
fn away_day(n: u32) -> Day {
    let last = Day::new(YEAR - 1, 12, 31).unwrap_or_else(|_| refuse("a real day"));
    Day::from_days(last.days_from_epoch() - n).unwrap_or_else(|_| refuse("a day in range"))
}

/// One small stored day zip of `day`: its folders and two invented files.
fn day_zip(day: Day) -> Vec<u8> {
    let folder = day_folder_name(day);
    stored_zip(&[
        (format!("{folder}/"), Vec::new()),
        (format!("{folder}/Options/"), Vec::new()),
        (
            format!("{folder}/Options/INVENTEDA27AUG26100CE.NFO.csv"),
            b"Ticker,Date,Time,LTP\nINVENTEDA,01/01/2000,09:15:00,1.25\n".to_vec(),
        ),
        (
            format!("{folder}/Options/INVENTEDB27AUG26100PE.NFO.csv"),
            b"Ticker,Date,Time,LTP\nINVENTEDB,01/01/2000,09:15:00,2.50\n".to_vec(),
        ),
    ])
}

/// A year zip of `entries` entries in the mix the module header describes.
fn year_zip(entries: u32) -> Vec<u8> {
    let tenth = entries / 10;
    let mut members = Vec::new();
    for n in 0..tenth {
        let day = own_day(n);
        members.push((NfoZips::day_zip_name(day), day_zip(day)));
        let away = away_day(n);
        members.push((NfoZips::day_zip_name(away), day_zip(away)));
    }
    for n in 0..entries - 2 * tenth {
        let month = n % 12 + 1;
        let name = if n < 12 {
            format!("{month:02}_{YEAR}/")
        } else {
            format!("{month:02}_{YEAR}/INVENTED_{n}.txt")
        };
        members.push((name, b"x".to_vec()));
    }
    stored_zip(&members)
}

/// A folder holding one `<YEAR>.zip` of `entries` entries.
fn with_year_zip(entries: u32) -> PathBuf {
    let dir = folder(&entries.to_string());
    if std::fs::write(dir.join(format!("{YEAR}.zip")), year_zip(entries)).is_err() {
        refuse("the invented year zip written");
    }
    dir
}

/// Whether `source` lists `day` with its day zip's four entries.
fn lists(source: &NfoZips, day: Day) -> bool {
    matches!(source.day(day), Ok(Some(listing)) if listing.entries().len() == 4)
}

/// What a source over `dir` must answer before anything is timed.
fn check(dir: &Path, entries: u32) {
    let source = NfoZips::new(dir);
    let tenth = entries / 10;
    for n in [0, tenth / 2, tenth - 1] {
        if !lists(&source, own_day(n)) {
            refuse("an own-place day zip that does not list its four entries");
        }
    }
    // A day of the year before, held only inside this year's zip, refuses
    // as misfiled, naming where it was found (the year boundary crossed).
    if !matches!(source.day(away_day(0)), Err(CmRefusal::DayZipMisfiled { .. })) {
        refuse("a day packed into the next year's zip refuses as misfiled");
    }
    // A day of the year no zip holds is no day.
    if !matches!(source.day(own_day(tenth + 5)), Ok(None)) {
        refuse("a day no zip holds answers nothing");
    }
}

/// The extremes a run can meet before a single year zip: a folder that is
/// not there, an empty folder, and one day filed twice at its own place.
fn the_extremes_answer() {
    let day = own_day(0);
    let absent = std::env::temp_dir().join(format!(
        "brutex-chk-nfo-absent-{}-never-made",
        std::process::id()
    ));
    if !matches!(NfoZips::new(&absent).day(day), Ok(None)) {
        refuse("a folder that is not there holds no day");
    }
    let empty = folder("empty");
    if !matches!(NfoZips::new(&empty).day(day), Ok(None)) {
        refuse("an empty folder holds no day");
    }
    let twice = folder("twice");
    let doubled = stored_zip(&[
        (NfoZips::day_zip_name(day), day_zip(day)),
        (NfoZips::day_zip_name(day), day_zip(day)),
    ]);
    if std::fs::write(twice.join(format!("{YEAR}.zip")), doubled).is_err() {
        refuse("the doubled year zip written");
    }
    if !matches!(
        NfoZips::new(&twice).day(day),
        Err(CmRefusal::ArchiveDuplicateDay { .. })
    ) {
        refuse("a day filed twice at its own place refuses as a duplicate");
    }
}

/// The minimum, over [`FIRST_TRIALS`] fresh sources, of one first call, in
/// picoseconds.
fn first_call_ps(dir: &Path, day: Day) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..FIRST_TRIALS {
        let source = NfoZips::new(dir);
        let start = Instant::now();
        black_box(lists(&source, black_box(day)));
        best = best.min(start.elapsed().as_nanos() * 1_000);
    }
    best
}

/// C-CHK-02 and C-CHK-03: the run's first `day()` per year-zip entry, and
/// every later one, at 1x, 10x and 100x the entries.
pub(crate) fn year_index_is_flat() -> bool {
    the_extremes_answer();
    let probe = own_day(1);
    let mut first = Vec::with_capacity(3);
    let mut later = Vec::with_capacity(3);
    let mut largest = None;
    for entries in [BASE_ENTRIES, BASE_ENTRIES * 10, BASE_ENTRIES * 100] {
        let dir = with_year_zip(entries);
        check(&dir, entries);
        let once = first_call_ps(&dir, probe);
        println!(
            "  C-CHK-02 first day() at {entries:>5} year-zip entries: {} us per run",
            once / 1_000_000
        );
        first.push(once / u128::from(entries));
        let warm = NfoZips::new(&dir);
        if !lists(&warm, probe) {
            refuse("the warm source lists its probe day");
        }
        later.push(cost_ps(2_000, || lists(&warm, black_box(probe))));
        largest = Some((dir, warm, entries));
    }
    let ([f1, f10, f100], [l1, l10, l100]) = (first.as_slice(), later.as_slice()) else {
        refuse("three sizes measured")
    };
    let mut ok = linear_per_unit("C-CHK-02 first day() per year-zip entry", [*f1, *f10, *f100]);
    ok &= ratio("C-CHK-03 later day(), 10x year zip", *l1, *l10);
    ok &= ratio("C-CHK-03 later day(), 100x year zip", *l1, *l100);
    let Some((dir, warm, entries)) = largest else {
        refuse("the largest year zip")
    };
    spread(
        &format!("C-CHK-02 first day(), {entries} entries"),
        200,
        || {
            let source = NfoZips::new(&dir);
            black_box(lists(&source, black_box(probe)));
        },
    );
    let tenth = entries / 10;
    let mut at = 0;
    spread(
        &format!("C-CHK-03 later day(), {entries} entries"),
        2_000,
        || {
            let day = own_day(at % tenth);
            at += 1;
            black_box(lists(&warm, black_box(day)));
        },
    );
    ok
}
