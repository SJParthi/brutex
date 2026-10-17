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
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::instrument::{Exchange, Segment};

use super::{
    Carry, Code, Day, DayBlock, Entry, Facts, Feed, Generation, Key, Mode, Reader, Series,
    VerdictError, check_trailer, commit, day_of_year, days_from_civil, epoch_day, table,
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

/// The point [`refused_write`] answers at.
const DISK_FULL: &str = "tmp write refused";

/// The seam `write_durable` calls after its write under `cfg(test)`: when
/// [`DISK_FULL`] is armed on this thread, it disarms it and answers as a
/// host with no space left does, so the temporary already holds bytes.
pub(super) fn refused_write() -> io::Result<()> {
    if ARMED.with(|armed| armed.get() == Some(DISK_FULL)) {
        ARMED.with(|armed| armed.set(None));
        return Err(io::Error::new(
            ErrorKind::StorageFull,
            "no space left on device (the test seam)",
        ));
    }
    Ok(())
}

/// What another actor does to a writer's files at a named point: given the
/// temporary and the final name.
type Actor = fn(&Path, &Path);

thread_local! {
    /// The interference armed for this thread, if any: its point and actor.
    static INTERFERE: Cell<Option<(&'static str, Actor)>> = const { Cell::new(None) };
}

/// The seam `write_durable`, `remove_tmp`, `genesis` and `publish` call
/// before a host call under `cfg(test)`: when `at` is armed on this
/// thread, it disarms it and runs the actor, as another process would act
/// between two of the writer's calls, then the writer goes on.
pub(super) fn interfere(at: &'static str, tmp: &Path, path: &Path) {
    if let Some((armed, actor)) = INTERFERE.with(Cell::get)
        && armed == at
    {
        INTERFERE.with(|armed| armed.set(None));
        actor(tmp, path);
    }
}

/// Arms one interference for the next commit on this thread.
fn arm_interference(at: &'static str, actor: Actor) {
    INTERFERE.with(|armed| armed.set(Some((at, actor))));
}

/// Whether the armed interference is still waiting.
fn interference_armed() -> bool {
    INTERFERE.with(Cell::get).is_some()
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
            exchange: Exchange::Nse,
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
    assert_eq!(found.answer.unwrap().code(), Some(Code::Match));
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
            .any(|n| n.starts_with("2031-03-04.g0000000002.vdt.") && n.ends_with("-22.tmp")),
        "the crashed writer's temporary is left: {left:?}"
    );
    assert!(!year_dir.join("2031-03-04.g0000000002.vdt").exists());
    let mut reader = Reader::new();
    let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
    assert_eq!(
        answer.code(),
        Some(Code::Match),
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
        check_trailer(&orphan).unwrap().generation,
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
    assert_eq!(answer.code(), Some(Code::CountMismatch));
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

/// The refusal of a write the host has no room for: the action and the kind.
fn disk_full(refused: &VerdictError) -> (&'static str, ErrorKind) {
    match refused {
        VerdictError::Io { action, source, .. } => (*action, source.kind()),
        other => panic!("expected the host's refusal, got {other}"),
    }
}

/// A full disk (the operator's rule 4): a write the host refuses removes
/// the temporary it had just made, at the year index's temporary and at a
/// day image's alike, so the refused commit leaves no partial file, the
/// committed generation still answers, and the next commit (room again)
/// finds nothing to clear. The refusal comes from the test seam after the
/// bytes were written, as a host that runs out part-way leaves them; a real
/// `ENOSPC` is not raised here.
#[test]
fn a_write_the_host_refuses_leaves_no_temporary() {
    let root = Root::new();
    let series = root.series();
    let dir = series.dir();
    let year_dir = dir.join("2031");
    let key = Key::instrument("NSE-FNO-ZZTEST-A").unwrap();
    let one = |run: u64, code: Code| generation(run, vec![entry("NSE-FNO-ZZTEST-A", code)]);

    // The year's first commit: the refused write is the index's temporary.
    arm(DISK_FULL);
    let refused = commit(&series, &one(21, Code::Match)).unwrap_err();
    assert_eq!(refused.word(), "io");
    assert_eq!(disk_full(&refused), ("write", ErrorKind::StorageFull));
    assert_eq!(
        names(&dir),
        ["2031", "2031.vdi.lock"],
        "no year index and no temporary of one is left"
    );
    assert_eq!(names(&year_dir), Vec::<String>::new());
    let mut reader = Reader::new();
    assert_eq!(reader.lookup(&series, day(), &key).unwrap().answer, None);

    let first = commit(&series, &one(22, Code::Match)).unwrap();
    assert_eq!((first.generation, first.staging_cleared), (1, 0));

    // The next commit: the refused write is the day image's temporary.
    arm(DISK_FULL);
    let refused = commit(&series, &one(23, Code::ValueMismatch)).unwrap_err();
    assert_eq!(disk_full(&refused), ("write", ErrorKind::StorageFull));
    assert_eq!(
        names(&year_dir),
        ["2031-03-04.g0000000001.vdt"],
        "the committed image alone: no temporary and no second generation"
    );
    assert_eq!(names(&dir), ["2031", "2031.vdi", "2031.vdi.lock"]);
    let held = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(held.answer.unwrap().code(), Some(Code::Match));

    let second = commit(&series, &one(24, Code::ValueMismatch)).unwrap();
    assert_eq!(
        (
            second.generation,
            second.previous,
            second.stepped_past,
            second.staging_cleared
        ),
        (2, 1, 0, 0),
        "nothing was left to step past or to clear"
    );
    let now = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(now.answer.unwrap().code(), Some(Code::ValueMismatch));
}

/// A real FIFO at `at`, made by `mkfifo` (test-only, as `tests/fifo.rs`).
fn mkfifo(at: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(at)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", at.display());
}

/// What `ask` answers on its own thread, with `root` as the series root;
/// a call still waiting after ten seconds is released by opening `pipe`'s
/// other end, and the test fails naming it.
fn within<T: Send + 'static>(
    root: &Path,
    pipe: &Path,
    ask: impl FnOnce(&Series<'_>) -> T + Send + 'static,
) -> T {
    let root = root.to_path_buf();
    let (send, answer) = std::sync::mpsc::channel();
    let _thread = std::thread::spawn(move || {
        let series = Series {
            root: &root,
            feed: Feed::Gdfl,
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            timeframe: Timeframe::SECOND_1,
        };
        send.send(ask(&series))
    });
    let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
        let _release = fs::OpenOptions::new().write(true).open(pipe);
        panic!("a verdict call waited on the pipe at {}", pipe.display());
    };
    answered
}

/// **A PIPE, A DEVICE OR A LINK AT A VERDICT FILE'S NAME IS REFUSED BY NAME,
/// AND NOTHING WAITS ON IT** (R3-16, RA-317; verify-IA-5-r2 follow-up 1).
/// The reader opened the year index and the day image with `O_NOFOLLOW`
/// alone, so a pipe planted at either name held `Reader::lookup` for ever.
/// One table over the three names a commit and a lookup open (the year
/// index, the image, the commit lock) and the kinds a name can hold that is
/// not a regular file: each is refused by what it is, a link with its own
/// word, within the wait, and once the real file is back the same reader
/// answers again.
#[test]
fn a_pipe_device_or_link_at_a_verdict_name_is_refused_and_nothing_waits() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZTEST-P").unwrap();
    let done = commit(
        &series,
        &generation(31, vec![entry("NSE-FNO-ZZTEST-P", Code::Match)]),
    )
    .unwrap();
    let names = [
        ("index", series.year_index(2031)),
        ("image", done.path.clone()),
        ("lock", series.lock_path(2031)),
    ];
    for (what, name) in names {
        let aside = name.with_extension("aside");
        fs::rename(&name, &aside).unwrap();
        for (kind, word) in [
            ("pipe", "not_regular"),
            ("directory", "not_regular"),
            ("link", "symlinked"),
            ("device", "not_regular"),
        ] {
            match kind {
                "pipe" => mkfifo(&name),
                "directory" => fs::create_dir(&name).unwrap(),
                "link" => std::os::unix::fs::symlink(&aside, &name).unwrap(),
                _ => std::os::unix::fs::symlink("/dev/null", &name).unwrap(),
            }
            // A lookup opens the index and the image, never the lock; a
            // commit opens the lock and the index, and writes a NEW image,
            // so it is not asked at the image's name.
            let mut refused: Vec<VerdictError> = Vec::new();
            if what != "lock" {
                let lookup_key = key.clone();
                let looked = within(&root.0, &name, move |series| {
                    Reader::new().lookup(series, day(), &lookup_key).map(|_| ())
                });
                refused.push(looked.unwrap_err());
            }
            if what != "image" {
                let committed = within(&root.0, &name, |series| {
                    commit(
                        series,
                        &generation(32, vec![entry("NSE-FNO-ZZTEST-P", Code::ValueMismatch)]),
                    )
                    .map(|_| ())
                });
                refused.push(committed.unwrap_err());
            }
            assert!(!refused.is_empty());
            for why in refused {
                // A device behind a link is the link: it is never followed.
                let word = if kind == "device" { "symlinked" } else { word };
                assert_eq!(why.word(), word, "{what} {kind}: {why}");
                assert!(
                    why.to_string().contains(&name.display().to_string()),
                    "{what} {kind}: {why}"
                );
                if let VerdictError::NotRegular { found, .. } = &why {
                    assert!(why.to_string().contains(found.word()), "{why}");
                }
            }
            if fs::symlink_metadata(&name).unwrap().is_dir() {
                fs::remove_dir(&name).unwrap();
            } else {
                fs::remove_file(&name).unwrap();
            }
        }
        fs::rename(&aside, &name).unwrap();
        let mut reader = Reader::new();
        let answer = reader.lookup(&series, day(), &key).unwrap().answer.unwrap();
        assert_eq!(answer.code(), Some(Code::Match), "{what}: answers again");
    }
    // Every refusal above wrote nothing: the next commit is generation 2.
    let next = commit(
        &series,
        &generation(33, vec![entry("NSE-FNO-ZZTEST-P", Code::ValueMismatch)]),
    )
    .unwrap();
    assert_eq!((next.generation, next.previous), (2, 1));
}

/// The refusal's action, the host's kind and the path it names.
fn io_refusal(refused: &VerdictError) -> (&'static str, ErrorKind, PathBuf) {
    match refused {
        VerdictError::Io {
            action,
            source,
            path,
        } => (*action, source.kind(), path.clone()),
        other => panic!("expected the host's refusal, got {other}"),
    }
}

/// The year's first temporary of the year index, as a writer of `run`
/// names it.
fn index_tmp(series: &Series<'_>, run: u64) -> PathBuf {
    let path = series.year_index(2031);
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(format!(".{}-{run}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// **A REFUSED WRITE WHOSE TEMPORARY ANOTHER ACTOR TOOK SAYS ONLY THAT THE
/// WRITE WAS REFUSED; ONE WHOSE NAME IT CANNOT REMOVE SAYS THAT TOO.**
///
/// Kills the three mutants of `write_durable`'s guard
/// `left.kind() != ErrorKind::NotFound` (to `true`, to `false`, and `!=` to
/// `==`): a full disk refuses the year index's temporary, and before the
/// writer removes it another actor has either removed it already (gone: the
/// refusal is the write's alone) or put a directory at its name (held: the
/// refusal also says the temporary could not be removed). Both arms are
/// reached only through a second actor between two calls, so the seam is
/// that actor.
#[test]
fn a_refused_writes_temporary_gone_or_held_is_said_as_it_is() {
    let root = Root::new();
    let series = root.series();
    let one = |run: u64| generation(run, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]);

    arm(DISK_FULL);
    arm_interference("refused tmp removed", |tmp, _| {
        fs::remove_file(tmp).unwrap();
    });
    let refused = commit(&series, &one(31)).unwrap_err();
    assert!(!interference_armed());
    assert_eq!(
        io_refusal(&refused),
        ("write", ErrorKind::StorageFull, index_tmp(&series, 31))
    );

    arm(DISK_FULL);
    arm_interference("refused tmp removed", |tmp, _| {
        fs::remove_file(tmp).unwrap();
        fs::create_dir(tmp).unwrap();
    });
    let refused = commit(&series, &one(32)).unwrap_err();
    assert!(!interference_armed());
    assert_eq!(
        io_refusal(&refused),
        (
            "write (its temporary could not be removed)",
            ErrorKind::StorageFull,
            index_tmp(&series, 32)
        )
    );
    assert!(
        index_tmp(&series, 32).is_dir(),
        "what held the name is left"
    );
    fs::remove_dir(index_tmp(&series, 32)).unwrap();
    assert_eq!(commit(&series, &one(33)).unwrap().generation, 1);
}

/// **A TEMPORARY ALREADY GONE AFTER ITS LINK IS REMOVED; ONE WHOSE NAME
/// CANNOT BE REMOVED REFUSES THE COMMIT AS A REMOVE.**
///
/// Kills `remove_tmp`'s guard `why.kind() != ErrorKind::NotFound` to
/// `true`, to `false`, and `!=` to `==`: after the year index is linked
/// another actor removes its temporary (the commit goes on: nothing is left
/// to remove), or puts a directory at its name (the commit is refused as
/// the `remove` of that name, naming it). The index is linked either way,
/// so the next commit finds it and lands.
#[test]
fn a_linked_temporary_gone_is_done_and_one_held_refuses_as_a_remove() {
    let root = Root::new();
    let series = root.series();
    let one = |run: u64| generation(run, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]);

    arm_interference("tmp removed", |tmp, _| fs::remove_file(tmp).unwrap());
    assert_eq!(commit(&series, &one(41)).unwrap().generation, 1);
    assert!(!interference_armed());

    let other = Root::new();
    let series = other.series();
    arm_interference("tmp removed", |tmp, _| {
        fs::remove_file(tmp).unwrap();
        fs::create_dir(tmp).unwrap();
    });
    let refused = commit(&series, &one(42)).unwrap_err();
    assert!(!interference_armed());
    let (action, _, path) = io_refusal(&refused);
    assert_eq!((action, path), ("remove", index_tmp(&series, 42)));
    assert!(series.year_index(2031).exists(), "the index was linked");
    fs::remove_dir(index_tmp(&series, 42)).unwrap();
    assert_eq!(commit(&series, &one(43)).unwrap().generation, 1);
}

/// **A YEAR INDEX ANOTHER WRITER LINKED FIRST IS READ; A LINK THE HOST
/// REFUSES OTHERWISE IS THE LINK'S REFUSAL, AND NOTHING IS LEFT.**
///
/// Kills `genesis`' guard `why.kind() == ErrorKind::AlreadyExists` to
/// `true` and to `false`, and `==` to `!=`. Between the writer's temporary
/// and its link, another actor either links an identical index at the name
/// first (the creation race's loser reads the winner's, and the commit
/// lands at generation 1), or takes the temporary away (`ENOENT` at the
/// link: refused as the `link` of the year index, and neither the index nor
/// a temporary is left).
#[test]
fn a_year_index_linked_first_is_read_and_any_other_refused_link_is_said() {
    let root = Root::new();
    let series = root.series();
    let one = |run: u64| generation(run, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]);

    arm_interference("index linked", |tmp, path| {
        fs::hard_link(tmp, path).unwrap();
    });
    assert_eq!(commit(&series, &one(51)).unwrap().generation, 1);
    assert!(!interference_armed());
    assert!(!index_tmp(&series, 51).exists());

    let other = Root::new();
    let series = other.series();
    arm_interference("index linked", |tmp, _| fs::remove_file(tmp).unwrap());
    let refused = commit(&series, &one(52)).unwrap_err();
    assert!(!interference_armed());
    assert_eq!(
        io_refusal(&refused),
        ("link", ErrorKind::NotFound, series.year_index(2031))
    );
    assert_eq!(names(&series.dir()), ["2031", "2031.vdi.lock"]);
}

/// **AN IMAGE LINK THE HOST REFUSES OTHER THAN "EXISTS" IS THE LINK'S
/// REFUSAL, NEVER A GENERATION TO STEP PAST.**
///
/// Kills `publish`'s guard `why.kind() == ErrorKind::AlreadyExists` to
/// `true`: every link refusal would be taken for a generation a crashed
/// writer left, and the writer would step on to the next one and publish
/// there. Between the image's temporary and its link another actor takes
/// the temporary away: the commit is refused as the `link` of generation
/// 2's image, the committed generation 1 still answers, and no image of
/// generation 2 or 3 exists.
#[test]
fn an_image_link_refused_otherwise_than_exists_is_never_stepped_past() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZTEST-A").unwrap();
    commit(
        &series,
        &generation(61, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]),
    )
    .unwrap();
    arm_interference("image linked", |tmp, _| fs::remove_file(tmp).unwrap());
    let refused = commit(
        &series,
        &generation(62, vec![entry("NSE-FNO-ZZTEST-A", Code::ValueMismatch)]),
    )
    .unwrap_err();
    assert!(!interference_armed());
    assert_eq!(
        io_refusal(&refused),
        ("link", ErrorKind::NotFound, series.image_path(day(), 2))
    );
    assert_eq!(
        names(&series.dir().join("2031")),
        ["2031-03-04.g0000000001.vdt"]
    );
    let mut reader = Reader::new();
    let held = reader.lookup(&series, day(), &key).unwrap();
    assert_eq!(held.answer.unwrap().code(), Some(Code::Match));
}

/// **A LOCK THE HOST REFUSES OTHER THAN "HELD" IS THE LOCK'S REFUSAL, AT
/// ONCE, AND NOTHING IS WRITTEN.**
///
/// Covers `lock_within`'s arm for a `flock` refused other than
/// `EWOULDBLOCK`, which no lock file gives on request: the flock test seam
/// refuses the commit's one try (`EACCES` standing in for `ENOLCK`). The
/// refusal is `io` with the action `lock`, naming the year's lock file, and
/// it is not waited on (a held lock is waited on, and only it). No index
/// and no image is made.
#[test]
fn a_lock_the_host_refuses_is_the_locks_refusal_and_nothing_is_written() {
    let root = Root::new();
    let series = root.series();
    crate::flock::tests::refuse_next_lock(ErrorKind::PermissionDenied);
    let refused = commit(
        &series,
        &generation(71, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]),
    )
    .unwrap_err();
    assert!(!crate::flock::tests::lock_refusal_armed());
    let (action, kind, path) = io_refusal(&refused);
    assert_eq!((action, kind), ("lock", ErrorKind::PermissionDenied));
    assert_eq!(path.file_name().unwrap(), "2031.vdi.lock");
    assert!(!series.year_index(2031).exists(), "no index");
    assert_eq!(names(&series.dir().join("2031")), Vec::<String>::new());
}

/// **A YEAR INDEX BORN BETWEEN A READER'S TWO LOOKS IS READ, NEVER CALLED
/// LOST.**
///
/// Covers `index_handle`'s second look succeeding: the reader finds no
/// index, finds a day image in the year directory (so the index would be
/// lost), and looks once more before saying so; the index is there by
/// then. Staged here by moving a committed index aside and back between the
/// two looks. The lookup answers the committed code at the cost of the
/// extra look: the index opened on the second try, the directory listed,
/// the image opened.
#[test]
fn a_year_index_born_between_a_readers_two_looks_is_read() {
    let root = Root::new();
    let series = root.series();
    let key = Key::instrument("NSE-FNO-ZZTEST-A").unwrap();
    commit(
        &series,
        &generation(81, vec![entry("NSE-FNO-ZZTEST-A", Code::Match)]),
    )
    .unwrap();
    let index = series.year_index(2031);
    let aside = index.with_extension("vdi.aside");
    fs::rename(&index, &aside).unwrap();
    arm_interference("index looked for again", |path, _| {
        fs::rename(path.with_extension("vdi.aside"), path).unwrap();
    });
    let mut reader = Reader::new();
    let found = reader.lookup(&series, day(), &key).unwrap();
    assert!(!interference_armed());
    assert_eq!(found.answer.unwrap().code(), Some(Code::Match));
    assert_eq!(found.cost.opens, 3, "the index, the directory, the image");

    // The same, with no index coming: the lost index's refusal.
    fs::rename(&index, &aside).unwrap();
    let mut reader = Reader::new();
    assert_eq!(
        reader.lookup(&series, day(), &key).unwrap_err().word(),
        "index_missing"
    );
}
