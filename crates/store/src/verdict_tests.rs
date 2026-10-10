//! Unit tests of `store::verdict` that need the crash seam: a writer stopped
//! at a named point leaves the disk exactly as a killed process would, and
//! the next reader and writer must cope with what it left (D-4865;
//! IA-VD-05, IA-VD-09, IA-VD-10). Every fixture is invented: keys name no
//! real contract and every count is arbitrary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::instrument::Segment;

use super::{
    Carry, Code, Day, DayBlock, Entry, Facts, Feed, Generation, Key, Mode, Reader, Series,
    VerdictError, commit, day_of_year, days_from_civil, epoch_day, scrub, table,
};
use crate::path::Timeframe;

thread_local! {
    /// The crash point armed for this thread, if any.
    static ARMED: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The seam `commit` calls at each named point under `cfg(test)`: when the
/// point is armed on this thread, it disarms it and stops the writer there.
pub(super) fn crash(at: &'static str) -> Result<(), VerdictError> {
    if ARMED.with(|armed| armed.get() == Some(at)) {
        ARMED.with(|armed| armed.set(None));
        return Err(VerdictError::Crashed { at });
    }
    Ok(())
}

/// Arms one crash point for the next commit on this thread.
fn arm(at: &'static str) {
    ARMED.with(|armed| armed.set(Some(at)));
}

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-vd-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn series(&self) -> Series<'_> {
        Series {
            root: &self.0,
            feed: Feed::Gdfl,
            segment: Segment::Fno,
            timeframe: Timeframe::SECOND_1,
        }
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn day() -> Day {
    Day::new(2031, 3, 4).unwrap()
}

fn entry(name: &str, code: Code) -> Entry {
    Entry {
        key: Key::instrument(name).unwrap(),
        facts: Facts::of(code),
        details: Vec::new(),
    }
}

fn generation(run: u64, entries: Vec<Entry>) -> Generation {
    Generation {
        day: day(),
        run,
        written_at: 1_900_000_000,
        verifier: 1,
        mode: Mode::Cold,
        rule_set: 0,
        origin: [7; 16],
        day_block: DayBlock::of(Code::Match),
        day_details: Vec::new(),
        entries,
        carry: Carry::WholeDay,
    }
}

/// Every file name in a directory, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .map(|it| {
            it.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// IA-VD-10: a writer killed after writing the year index's temporary and
/// before linking it leaves no `.vdi` in place, only its own temporary,
/// which no reader opens; the next writer builds the index whole.
#[test]
fn a_year_index_is_born_whole() {
    let root = Root::new();
    let series = root.series();
    arm("index tmp written");
    let refused = commit(
        &series,
        &generation(11, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]),
    );
    assert_eq!(refused.unwrap_err().word(), "crashed");
    let dir = series.dir();
    let left = names(&dir);
    assert!(
        !left.iter().any(|n| n == "2031.vdi"),
        "a half-made year index is in place: {left:?}"
    );
    assert!(
        left.iter()
            .any(|n| n.starts_with("2031.vdi.") && n.ends_with("-11.tmp")),
        "the writer's own temporary is what a crash leaves: {left:?}"
    );
    let mut reader = Reader::new();
    let key = Key::instrument("NSE-FNO-ZZTEST-A").unwrap();
    let never = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(
        never.answer, None,
        "nothing was committed, so nothing is read"
    );

    let done = commit(
        &series,
        &generation(12, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]),
    )
    .unwrap();
    assert_eq!(done.generation, 1);
    let len = fs::metadata(series.year_index(2031)).unwrap().len();
    assert_eq!(len, 32_768, "the index is born at its whole length");
    let found = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(found.answer.unwrap().code(), Code::Match);
}

/// IA-VD-05: a writer killed after writing an image's temporary and before
/// linking it leaves the temporary, which no lookup reads; the committed
/// generation still answers, and the next writer publishes past it.
#[test]
fn a_temporary_day_image_is_never_read_unit() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZTEST-B").unwrap();
    commit(
        &series,
        &generation(21, vec![entry("NSE-FNO-ZZTEST-B", Code::Match)]),
    )
    .unwrap();
    arm("image tmp written");
    let refused = commit(
        &series,
        &generation(22, vec![entry("NSE-FNO-ZZTEST-B", Code::ValueMismatch)]),
    );
    assert_eq!(refused.unwrap_err().word(), "crashed");
    let year_dir = series.dir().join("2031");
    let left = names(&year_dir);
    assert!(
        left.iter()
            .any(|n| n.ends_with(".g000002.vdt.") || n.contains(".g000002.vdt.")),
        "the crashed writer's temporary is left: {left:?}"
    );
    assert!(!year_dir.join("2031-03-04.g000002.vdt").exists());
    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(
        answer.code(),
        Code::Match,
        "the temporary's mismatch is never read"
    );
    assert_eq!(answer.header.generation, 1);
}

/// IA-VD-09: a writer killed after linking generation 2 and before
/// committing the slot leaves image 2 linked and uncommitted. Readers still
/// see generation 1; the next writer steps past 2 to 3, and image 2 is kept
/// byte for byte.
#[test]
fn a_crash_between_link_and_commit_never_overwrites_an_image_unit() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZTEST-C").unwrap();
    commit(
        &series,
        &generation(31, vec![entry("NSE-FNO-ZZTEST-C", Code::Match)]),
    )
    .unwrap();
    arm("image linked");
    let refused = commit(
        &series,
        &generation(32, vec![entry("NSE-FNO-ZZTEST-C", Code::ExtraInStore)]),
    );
    assert_eq!(refused.unwrap_err().word(), "crashed");
    let orphan = series.image_path(day(), 2);
    let before = fs::read(&orphan).unwrap();
    assert_eq!(
        scrub(&orphan).unwrap().generation,
        2,
        "the orphan is a whole image"
    );

    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(
        answer.header.generation, 1,
        "an uncommitted image is not read"
    );

    let done = commit(
        &series,
        &generation(33, vec![entry("NSE-FNO-ZZTEST-C", Code::CountMismatch)]),
    )
    .unwrap();
    assert_eq!(done.generation, 3);
    assert_eq!(done.stepped_past, 1);
    assert_eq!(done.previous, 1);
    assert_eq!(
        fs::read(&orphan).unwrap(),
        before,
        "image 2 is never overwritten"
    );
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(answer.header.generation, 3);
    assert_eq!(answer.code(), Code::CountMismatch);
}

/// The seam fires once and only at its own point.
#[test]
fn the_crash_seam_fires_once_at_its_own_point() {
    assert!(crash("image linked").is_ok(), "an unarmed point passes");
    arm("image linked");
    assert!(crash("image tmp written").is_ok(), "another point passes");
    assert_eq!(crash("image linked").unwrap_err().word(), "crashed");
    assert!(crash("image linked").is_ok(), "it disarms itself");
}

/// The civil-day arithmetic against dates whose day counts are fixed:
/// 1970-01-01 is day 0, 2000-03-01 is day 11,017, and leap years give 1
/// January of the next year at 366.
#[test]
fn days_are_counted_from_the_epoch_and_the_year() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(days_from_civil(1969, 12, 31), -1);
    assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    assert_eq!(days_from_civil(2000, 2, 29), 11_016);
    assert_eq!(epoch_day(Day::new(1970, 1, 2).unwrap()).unwrap(), 1);
    assert_eq!(
        epoch_day(Day::new(1969, 12, 31).unwrap())
            .unwrap_err()
            .word(),
        "day_out_of_range"
    );
    assert_eq!(day_of_year(Day::new(2024, 1, 1).unwrap()), 0);
    assert_eq!(day_of_year(Day::new(2024, 12, 31).unwrap()), 365);
    assert_eq!(day_of_year(Day::new(2023, 12, 31).unwrap()), 364);
    assert_eq!(day_of_year(Day::new(2023, 3, 1).unwrap()), 59);
    assert_eq!(day_of_year(Day::new(2024, 3, 1).unwrap()), 60);
}

/// The table's two rules, on chosen hashes: nine keys sharing one home slot
/// can never fit an 8-slot window at any capacity, and two keys with equal
/// hashes can never be told apart by tag.
#[test]
fn a_table_that_cannot_hold_its_keys_is_refused_by_name() {
    let crowded: Vec<u64> = (1..=9u64).map(|tag| tag << 32).collect();
    assert_eq!(table(&crowded).unwrap_err().word(), "table_too_large");
    let eight: Vec<u64> = (1..=8u64).map(|tag| tag << 32).collect();
    let (capacity, slots) = table(&eight).unwrap();
    assert_eq!(capacity, 16);
    assert_eq!(slots.len(), 16 + 7);
    assert!(
        slots[..8].iter().all(|(mark, _)| *mark != 0),
        "eight keys fill one window"
    );
    let twins = vec![5u64 << 32 | 0x03, 5u64 << 32 | 0x03];
    assert_eq!(table(&twins).unwrap_err().word(), "table_too_large");
    // Equal tags two slots apart are separated by doubling: at 16 they share
    // a window, at 32 and above they do not (homes 3 and 19).
    let near = vec![9u64 << 32 | 0x03, 9u64 << 32 | 0x13];
    let (capacity, _) = table(&near).unwrap();
    assert_eq!(capacity, 32);
}
