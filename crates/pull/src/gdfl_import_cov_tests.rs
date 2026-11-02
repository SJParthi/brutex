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

use std::path::{Path, PathBuf};

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
    assert!(
        zero.fault
            .is_some_and(|why| why.contains("more than 0 entries"))
    );
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
    for reason in [
        "needs_month_generation",
        "month_superseded",
        "repair_refused",
    ] {
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
    assert!(
        imported_days(&store, ImportKind::Indices)
            .unwrap()
            .is_empty()
    );
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
    assert!(
        imported_days(&store, ImportKind::Options)
            .unwrap()
            .is_empty()
    );
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
    let journal =
        format!("done indices {sunday} *\ndone indices {missing} *\ndone indices {held} *\n");
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
    assert_eq!(
        done.days_not_imported, 1,
        "June 1st, a Saturday, was never imported"
    );
    assert_eq!((done.files, done.files_refused), (1, 2), "{done:?}");
    let named: Vec<(String, String)> = done
        .failures
        .iter()
        .map(|f| (f.instrument.clone(), f.why.clone()))
        .collect();
    let calendar = format!("{sunday} calendar");
    let closed = |(what, _): &&(String, String)| *what == calendar;
    assert_eq!(named.iter().filter(closed).count(), 1, "{named:?}");
    let past = format!("{missing} past.csv");
    let stamped = |(what, why): &&(String, String)| *what == past && why.contains("86400");
    assert_eq!(named.iter().filter(stamped).count(), 1, "{named:?}");
    let member = format!("{missing} member.csv");
    let refused = |(what, why): &&(String, String)| *what == member && why == "an invented refusal";
    assert_eq!(named.iter().filter(refused).count(), 1, "{named:?}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A directory entry that answers what a test says: the real entry's type
/// (`kind_of`) or a failing disk's error, and the bytes it was told.
struct SaidEntry {
    name: std::ffi::OsString,
    path: PathBuf,
    kind_of: Option<PathBuf>,
    bytes: Option<u64>,
}

impl SaidEntry {
    /// A named entry of `dir` with the type of the real path `kind_of`.
    fn new(dir: &Path, name: &str, kind_of: Option<&Path>, bytes: Option<u64>) -> Self {
        Self {
            name: name.into(),
            path: dir.join(name),
            kind_of: kind_of.map(Path::to_path_buf),
            bytes,
        }
    }
}

impl OriginEntry for SaidEntry {
    fn kind(&self) -> std::io::Result<std::fs::FileType> {
        match &self.kind_of {
            Some(real) => std::fs::symlink_metadata(real).map(|meta| meta.file_type()),
            None => Err(std::io::Error::other("said: no type")),
        }
    }
    fn name(&self) -> std::ffi::OsString {
        self.name.clone()
    }
    fn location(&self) -> PathBuf {
        self.path.clone()
    }
    fn bytes(&self) -> std::io::Result<u64> {
        self.bytes
            .ok_or_else(|| std::io::Error::other("said: deleted in the walk"))
    }
}

/// The walk over `listed`, one directory, with a good `FORMAT.md` in `root`.
fn walk(root: &Path, cap: usize, listed: Vec<std::io::Result<SaidEntry>>) -> (Measured, u32) {
    let once = std::cell::RefCell::new((Some(listed), 0_u32));
    let measured = walk_tick_store(root, cap, &|_dir: &Path| {
        let mut state = once.borrow_mut();
        state.1 += 1;
        Ok(state.0.take().unwrap_or_default().into_iter())
    });
    let opened = once.borrow().1;
    (measured, opened)
}

/// A real directory and a real file, for the types a said entry answers, and
/// a good `FORMAT.md`.
fn walk_root(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(tag);
    let file = put(&root, "FORMAT.md", b"# a format of no store\n");
    let dir = root.join("a-dir");
    std::fs::create_dir_all(&dir).unwrap();
    (root, dir, file)
}

/// A listing that fails mid-way stops the walk with the directory named and
/// the bytes so far kept; no digest is claimed for a store not wholly read,
/// though `FORMAT.md` reads, and the directory found before it is never
/// opened (the walk stops, it does not limp on).
#[test]
fn a_listing_that_fails_stops_the_walk_and_names_the_directory() {
    let (root, dir, file) = walk_root("cov-walk-list");
    let (measured, opened) = walk(
        &root,
        10,
        vec![
            Ok(SaidEntry::new(&root, "sub", Some(&dir), None)),
            Ok(SaidEntry::new(&root, "a.bts", Some(&file), Some(7))),
            Err(std::io::Error::other("a failing disk")),
            Ok(SaidEntry::new(&root, "b.bts", Some(&file), Some(100))),
        ],
    );
    assert_eq!(measured.bytes, 7, "the bytes read before the fault stand");
    assert_eq!(measured.digest, None);
    assert_eq!(
        measured.fault,
        Some(format!("{}: a failing disk", root.display()))
    );
    assert_eq!(opened, 1, "the directory found first is not walked");
    std::fs::remove_dir_all(root).unwrap();
}

/// An entry whose type cannot be read is the same stop, named by the
/// directory holding it.
#[test]
fn an_entry_of_unknown_type_stops_the_walk() {
    let (root, _dir, file) = walk_root("cov-walk-kind");
    let (measured, _) = walk(
        &root,
        10,
        vec![
            Ok(SaidEntry::new(&root, "a.bts", None, Some(5))),
            Ok(SaidEntry::new(&root, "b.bts", Some(&file), Some(5))),
        ],
    );
    assert_eq!(measured.bytes, 0);
    assert_eq!(measured.digest, None);
    assert_eq!(
        measured.fault,
        Some(format!("{}: said: no type", root.display()))
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A file deleted between the listing and its size is named by its own path,
/// not its directory: that is what an operator can go and look at.
#[test]
fn a_file_gone_between_listing_and_size_names_the_file() {
    let (root, _dir, file) = walk_root("cov-walk-bytes");
    let (measured, _) = walk(
        &root,
        10,
        vec![
            Ok(SaidEntry::new(&root, "a.bts", Some(&file), Some(3))),
            Ok(SaidEntry::new(&root, "gone.bts", Some(&file), None)),
        ],
    );
    assert_eq!(measured.bytes, 3);
    assert_eq!(measured.digest, None);
    let fault = measured.fault.unwrap();
    assert!(
        fault.ends_with("gone.bts: said: deleted in the walk"),
        "{fault}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Combinations of extremes: the cap is asked before the entry, so a sick
/// entry past the cap is the cap's fault; and with no `FORMAT.md` as well the
/// first fault stands. An entry of `u64::MAX` bytes saturates, never wraps.
#[test]
fn the_walks_faults_keep_their_order_and_a_huge_size_saturates() {
    let (root, dir, file) = walk_root("cov-walk-combo");
    let (capped, _) = walk(&root, 0, vec![Err(std::io::Error::other("never read"))]);
    assert!(
        capped
            .fault
            .is_some_and(|fault| fault.contains("more than 0 entries")),
    );
    std::fs::remove_file(&file).unwrap();
    let kept = root.join("kept-type");
    std::fs::write(&kept, b"x").unwrap();
    let (sick, _) = walk(
        &root,
        5,
        vec![
            Ok(SaidEntry::new(&root, "gone.bts", Some(&kept), None)),
            Ok(SaidEntry::new(&root, "sub", Some(&dir), None)),
        ],
    );
    assert!(
        sick.fault
            .is_some_and(|fault| fault.ends_with("said: deleted in the walk")),
        "the missing FORMAT.md does not replace the first fault"
    );
    std::fs::write(&file, b"# again\n").unwrap();
    let (huge, _) = walk(
        &root,
        5,
        vec![
            Ok(SaidEntry::new(&root, "a.bts", Some(&kept), Some(u64::MAX))),
            Ok(SaidEntry::new(&root, "b.bts", Some(&kept), Some(u64::MAX))),
        ],
    );
    assert_eq!(huge.bytes, u64::MAX);
    assert_eq!(huge.fault, None);
    assert!(huge.digest.is_some());
    std::fs::remove_dir_all(root).unwrap();
}

/// A directory of yearly zips that fails mid-listing refuses the measure
/// (no partial list of years is ever summed); entries that are not years,
/// three-digit, five-digit or lettered, are passed over without a fault.
#[test]
fn yearly_zips_are_read_whole_or_refused() {
    let dir = Path::new("/invented");
    let said = |name: &str| Ok(SaidEntry::new(dir, name, None, None));
    let years = yearly_zips(
        vec![
            said("2019.zip"),
            said("201.zip"),
            said("20190.zip"),
            said("20x9.zip"),
            said("2019.ZIP"),
            said(".zip"),
            said("2020.zip"),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(
        years,
        [
            ("2019.zip".to_owned(), dir.join("2019.zip")),
            ("2020.zip".to_owned(), dir.join("2020.zip")),
        ]
    );
    let refused = yearly_zips(
        vec![
            said("2019.zip"),
            Err(std::io::Error::other("a failing disk")),
        ]
        .into_iter(),
    )
    .unwrap_err();
    assert_eq!(refused.to_string(), "a failing disk");
}

/// A `statfs` answer that overflows a `u64` is named with the volume and the
/// count, for each of the two counts; the largest product that fits is
/// answered exactly, and a volume of nothing is zero, not a refusal.
#[test]
fn a_volume_count_that_overflows_is_named_not_wrapped() {
    let at = Path::new("/invented/volume");
    let refused = space_of(at, 2, u64::MAX, 1).unwrap_err();
    assert_eq!(
        refused,
        format!(
            "/invented/volume: {} total blocks of 2 bytes overflow a u64",
            u64::MAX
        )
    );
    let refused = space_of(at, 4096, 1, u64::MAX / 2).unwrap_err();
    assert!(
        refused.contains("available blocks of 4096 bytes"),
        "{refused}"
    );
    let fits = space_of(at, 1, u64::MAX, u64::MAX).unwrap();
    assert_eq!((fits.total, fits.free), (u64::MAX, u64::MAX));
    let none = space_of(at, 0, u64::MAX, u64::MAX).unwrap();
    assert_eq!((none.total, none.free), (0, 0));
}

/// A contract directory settles a row only when it reads: an entry that will
/// not read counts as live (unread is not proof of absence), a name that is
/// no contract, or not UTF-8, is nothing to split, and so is a symbol the
/// store cannot spell. An empty directory has nothing live.
#[test]
fn an_unread_entry_of_an_old_symbol_counts_as_live() {
    use std::os::unix::ffi::OsStrExt as _;
    let root = scratch("cov-live");
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    let withdrawn = crate::gdfl_rekey::withdrawn(&store).unwrap();
    let symbol = brutex_core::symbol::Symbol::new("INVENTEDX").ok();
    let dir = Path::new("/invented");
    let said = |name: &str| Ok(SaidEntry::new(dir, name, None, None));
    let contract = "2020-09-24-15000-CE";
    assert!(!holds_a_live_contract(
        Vec::<std::io::Result<SaidEntry>>::new().into_iter(),
        symbol,
        &withdrawn
    ));
    assert!(holds_a_live_contract(
        vec![Err(std::io::Error::other("a failing disk")), said(contract)].into_iter(),
        symbol,
        &withdrawn
    ));
    assert!(holds_a_live_contract(
        vec![said("not-a-contract"), said(contract)].into_iter(),
        symbol,
        &withdrawn
    ));
    assert!(!holds_a_live_contract(
        vec![said("not-a-contract")].into_iter(),
        symbol,
        &withdrawn
    ));
    assert!(!holds_a_live_contract(
        vec![said(contract)].into_iter(),
        None,
        &withdrawn
    ));
    let mut bad = SaidEntry::new(dir, "x", None, None);
    bad.name = std::ffi::OsStr::from_bytes(&[0xFF, 0xFE]).to_os_string();
    assert!(!holds_a_live_contract(
        vec![Ok(bad)].into_iter(),
        symbol,
        &withdrawn
    ));
    std::fs::remove_dir_all(root).unwrap();
}

/// Only a row that applies, moved its symbol and is wanted can split a run.
/// A file where an old symbol's directory should be cannot be read, so the
/// run is held on it: unread is not proof of absence.
#[test]
fn a_lineage_row_splits_a_run_only_when_it_can_and_a_file_in_its_place_holds_the_run() {
    use brutex_core::alias::{LINEAGE, Status};
    let root = scratch("cov-names-split");
    let store = root.join("S");
    let on = day(2024, 3, 1);
    let wanted = ["TMPV".to_owned()];
    let run = |only| Run {
        kind: ImportKind::Options,
        from: on,
        to: on,
        only,
        store_root: &store,
    };
    let row = LINEAGE
        .rows()
        .iter()
        .find(|row| row.old == "TATAMOTORS")
        .unwrap();
    let mut unsourced = *row;
    unsourced.status = Status::Unverified("no exchange source read");
    assert!(run(&wanted).names_a_split(row, "TMPV"));
    assert!(run(&[]).names_a_split(row, "TMPV"));
    assert!(!run(&wanted).names_a_split(&unsourced, "TMPV"));
    assert!(
        !run(&wanted).names_a_split(row, "TATAMOTORS"),
        "never moved"
    );
    assert!(
        !run(&wanted).names_a_split(row, "LTM"),
        "the filter names another"
    );
    put(
        &store,
        "bars/gdfl/NSE/FNO/TATAMOTORS",
        b"a file, not a directory",
    );
    assert_eq!(
        run(&wanted).split_underlyings(),
        Ok(vec![("TATAMOTORS".to_owned(), "TMPV".to_owned())])
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The longest name wins whichever order the names come in; a name the ticker
/// does not start with is passed over; no name at all is no answer.
#[test]
fn the_longest_prefix_does_not_depend_on_the_order_of_the_names() {
    let names = ["NIFTY", "NIFTYNXT50", "BANK"];
    assert_eq!(longest_prefix("NIFTYNXT5024", names), Some("NIFTYNXT50"));
    let turned = ["NIFTYNXT50", "NIFTY", "BANK"];
    assert_eq!(longest_prefix("NIFTYNXT5024", turned), Some("NIFTYNXT50"));
    assert_eq!(longest_prefix("TV18BRDCST", names), None);
    assert_eq!(longest_prefix("", ["", "A"]), Some(""));
    assert_eq!(longest_prefix("A", std::iter::empty()), None);
}

/// A band that traded at two prices or more says so; none, says nothing.
/// `u32::MAX` booked rows widens into the event, never wraps.
#[test]
fn an_unattributed_band_is_said_only_when_there_is_one() {
    assert!(unattributed_event("2024-03-01", "INVENTEDX", "X.csv", 0).is_none());
    let event = unattributed_event("2024-03-01", "INVENTEDX", "X.csv", u32::MAX).unwrap();
    assert_eq!(event.level(), telemetry::Level::Warn);
    assert_eq!(
        event.fields(),
        &[
            ("kind", telemetry::Value::from("stocks")),
            ("day", telemetry::Value::from("2024-03-01")),
            ("symbol", telemetry::Value::from("INVENTEDX")),
            ("file", telemetry::Value::from("X.csv")),
            ("booked_rows", telemetry::Value::from(u64::from(u32::MAX))),
            ("code", telemetry::Value::from("auction_band_unattributed")),
        ]
    );
    // Both paths of the door itself: nothing to say, and a warn to emit.
    note_unattributed(day(2024, 3, 1), "INVENTEDX", "X.csv", 0);
    note_unattributed(day(2024, 3, 1), "INVENTEDX", "X.csv", 1);
}

/// What each store or repair door's refusal means for the day: a page reads
/// these words, so every one is stated, and no two kinds share a sentence.
#[test]
fn every_doors_refusal_says_what_it_means_for_the_day() {
    use crate::ingest::FailKind as K;
    let kinds = [
        (K::Superseded, "served from a repair revision and this file"),
        (K::RepairRefused, "a later build or repair can lift"),
        (K::RepairImpossible, "refused the month for good"),
        (K::BatchRefused, "an importer defect"),
        (K::Sealed, "its successor generation was not filed"),
        (K::Other, "the store refused the file"),
    ];
    let mut said = std::collections::HashSet::new();
    for (kind, word) in kinds {
        let sentence = causes::door_said(kind);
        assert!(sentence.contains(word), "{kind:?}: {sentence}");
        assert!(said.insert(sentence), "{kind:?} shares a sentence");
    }
}

/// A stop says where it ended: on a day, or between two; and a refusal that
/// is no stop says only that.
#[test]
fn a_stop_says_where_it_ended_and_what_a_resume_reads() {
    let stopped = |day| ImportRefusal::Stopped {
        kind: ImportKind::Stocks,
        day,
        by: "an invented operator".to_owned(),
        requested: 1_700_000_000,
        done: Box::new(Report::default()),
    };
    let on = stopped(Some(day(2024, 3, 1))).to_string();
    assert!(
        on.contains("at a file boundary on 2024-03-01, after 0 day(s)"),
        "{on}"
    );
    assert!(
        on.contains("unless its journal line closed it done"),
        "{on}"
    );
    let between = stopped(None).to_string();
    assert!(between.contains("between two days"), "{between}");
    assert!(between.contains("the first day not done"), "{between}");
}

/// A refusal that carries what the run did reports it; each other reports
/// nothing. The lineage and durability refusals say their own words.
#[test]
fn a_refusal_reports_only_what_it_carries() {
    let done = || Box::new(Report::default());
    let on = day(2024, 3, 1);
    let carrying = [
        ImportRefusal::Durability {
            kind: ImportKind::Indices,
            day: on,
            why: "an invented fsync refusal".to_owned(),
            done: done(),
        },
        ImportRefusal::Stopped {
            kind: ImportKind::Indices,
            day: None,
            by: "x".to_owned(),
            requested: 0,
            done: done(),
        },
    ];
    for refusal in &carrying {
        assert!(refusal.report().is_some(), "{refusal:?}");
    }
    assert_eq!(
        carrying[0].to_string(),
        "indices 2024-03-01: an invented fsync refusal; the run stopped there after 0 day(s) imported"
    );
    let lineage = ImportRefusal::Lineage {
        why: "an invented ledger".to_owned(),
    };
    assert!(lineage.report().is_none());
    assert_eq!(
        lineage.to_string(),
        "the lineage ledger: an invented ledger"
    );
}

/// Each reader refusal has its one cause; a stage or a close that drifts is a
/// journal line a resume reads differently.
#[test]
fn every_reader_refusal_left_has_its_cause() {
    use crate::gdfl_cm::CmRefusal as C;
    use crate::gdfl_nfo::NfoRefusal as N;
    let line = 7;
    for (got, code) in [
        (cm_cause(&C::FolderUnknown), "folder_unknown"),
        (cm_cause(&C::ExtensionUnknown), "extension_unknown"),
        (cm_cause(&C::TickerMismatch { line }), "ticker_mismatch"),
        (cm_cause(&C::DateMismatch { line }), "date_mismatch"),
        (cm_cause(&C::MalformedRow { line }), "malformed_row"),
        (cm_cause(&C::RowsOverCap { cap: 1 }), "rows_over_cap"),
        (
            nfo_cause(&N::TickerAmbiguous { ticker: "T".into() }),
            "ticker_ambiguous",
        ),
        (nfo_cause(&N::TickerMismatch { line }), "ticker_mismatch"),
        (nfo_cause(&N::DateMismatch { line }), "date_mismatch"),
        (
            nfo_cause(&N::OpenInterestRefused { line }),
            "open_interest_refused",
        ),
        (nfo_cause(&N::RowsOverCap { cap: 1 }), "rows_over_cap"),
    ] {
        assert_eq!(
            (got.stage, got.code, got.closes),
            (Stage::Vendor, code, true)
        );
    }
    let open = nfo_cause(&N::MonthlyExpiryUnmeasured { ticker: "T".into() });
    assert_eq!(
        (open.stage, open.code, open.closes),
        (Stage::Calendar, "monthly_expiry_unmeasured", false)
    );
}

/// A hole is an open cause at stage `file` with a hole's code, and nothing
/// else: the same code at another stage, or closing, is none.
#[test]
fn a_hole_takes_all_three_of_its_conditions() {
    let code = crate::ingest::FailKind::Superseded.code();
    assert!(Cause::open(Stage::File, code).is_hole());
    assert!(!Cause::open(Stage::Journal, code).is_hole());
    assert!(!Cause::open(Stage::File, "an_invented_code").is_hole());
    assert!(!Cause::vendor(code).is_hole());
}

/// A day the journal holds and the source does not: a rescan names it missing
/// on both doors, and reads no file (a combination: an empty tick store under
/// a journal that says the day was imported).
#[test]
fn a_rescan_of_a_day_the_source_does_not_hold_names_it_missing() {
    let root = scratch("cov-rescan-gone");
    let store = root.join("S");
    let on = day(2019, 5, 30);
    let journal = format!("done indices {on} *\ndone options {on} *\n");
    put(&store, "imports/gdfl.journal", journal.as_bytes());
    let source = root.join("empty-source");
    std::fs::create_dir_all(&source).unwrap();
    let cm = rescan_cm(
        &TickStore::new(&source),
        ImportKind::Indices,
        (on, on),
        &store,
    )
    .unwrap();
    assert_eq!(cm.days_missing, [on]);
    assert_eq!(cm.files, 0);
    for lineage in [false, true] {
        let nfo = rescan_nfo(&NfoTickStore::new(&source), (on, on), &store, lineage).unwrap();
        assert_eq!(nfo.days_missing, [on], "lineage {lineage}");
        assert_eq!(nfo.files, 0);
    }
    std::fs::remove_dir_all(root).unwrap();
}
