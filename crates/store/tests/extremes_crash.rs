//! Crash, resume and concurrency extremes of the store's writer, written for
//! the cloud extremes round of 10 Nov 2026 (`EXTREMES-REPORT.md`).
//!
//! Three things here are new to `crates/store/tests`:
//!
//! 1. **Every crash point of one append, across every file of the month.**
//!    `fault.rs` walks torn header slots in memory and `tail_proof.rs` one
//!    crash between the checksum seal and the commit. Here one append is taken
//!    apart into the writes the writer issues, in its order (the records, the
//!    `.crc` seal, the `.tix` entries, the 64-byte header slot, then the
//!    `.srt` and `.sum` renames), and the month is materialised at points
//!    inside and between each of them: torn by one byte, mid-record, by half,
//!    one byte short. Each state must open through the reader door at exactly
//!    the committed prefix, pass the full checksum audit, and resume (the same
//!    batch offered again) to the bytes an uninterrupted append leaves. Three
//!    appends are taken apart: a month's first, the one that crosses the
//!    1,024-bar index threshold, and an ordinary one above it.
//! 2. **A real writer process killed by `SIGKILL`** at seven points of its
//!    run, each chosen by watching the bar file grow, and resumed.
//! 3. **Readers racing a writer** that opens, appends and closes per batch,
//!    and a reader of a month that has no `.lock` file while a writer appends.
//!
//! Every value is invented. A test named `finding_*` is `#[ignore]`d because
//! it FAILS on the branch it was written against; its doc comment is the
//! finding.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, HEADER_LEN, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Scratch and fixtures
// ===========================================================================

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A fresh directory under the host's temp dir, removed on drop.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("brutex-xc-{}-{tag}-{serial}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

const SYMBOL: u32 = 26_000;
const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
/// 2024-06-03 09:15 IST, a Monday.
const T0: i64 = 1_717_386_300_000_000;

/// The 375 bars of the session `day` days after 2024-06-03.
fn session(day: i64) -> Vec<Bar> {
    (0..375)
        .map(|i| {
            let seed = day * 1_000 + i;
            Bar {
                ts_micros: T0 + day * DAY + i * MINUTE,
                open: 2_000_000 + seed,
                high: 2_000_900 + seed,
                low: 1_999_100 + seed,
                close: 2_000_300 + seed,
                volume: 7 + seed,
                open_interest: OI_NULL,
            }
        })
        .collect()
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("a legal month"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn month_dir(root: &Path) -> PathBuf {
    month()
        .to_path_buf(root)
        .parent()
        .expect("a month directory")
        .to_path_buf()
}

/// Every regular file of the month directory, by name, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let Ok(entries) = fs::read_dir(month_dir(root)) else {
        return out;
    };
    for entry in entries {
        let entry = entry.expect("an entry");
        if entry.file_type().expect("a type").is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            out.insert(name, fs::read(entry.path()).expect("the file"));
        }
    }
    out
}

/// A snapshot without the lock file, whose holder note is not data.
fn data(snap: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    snap.iter()
        .filter(|(name, _)| name.as_str() != "2024-06.lock")
        .map(|(name, bytes)| (name.clone(), bytes.clone()))
        .collect()
}

/// Writes `files` as the whole month directory of a fresh `root`.
fn materialise(root: &Path, files: &BTreeMap<String, Vec<u8>>) {
    let dir = month_dir(root);
    fs::create_dir_all(&dir).expect("the month directory");
    for (name, bytes) in files {
        fs::write(dir.join(name), bytes).expect("a month file");
    }
}

/// Every committed record of the month through the reader door.
fn read_back(root: &Path) -> Result<Vec<Bar>, StoreError> {
    let file = BarFile::open_existing(root, month(), SYMBOL)?;
    (0..file.records()).map(|i| file.read_record(i)).collect()
}

fn audit(root: &Path) -> Result<(), String> {
    BarFile::open_existing_audited(root, month(), SYMBOL, u64::MAX).map(drop)
}

// ===========================================================================
// 1. Every crash point of one append
// ===========================================================================

/// The writes one append issues, in the writer's order
/// (`crates/store/src/file.rs`, `BarFile::append`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    /// The records, written past the old end of the `.bin`.
    Records,
    /// The `.crc` seal of the touched blocks.
    Checksums,
    /// The `.tix` entries.
    TimeIndex,
    /// The 64-byte header slot inside the `.bin` header region.
    Slot,
    /// The `.srt`, published by rename.
    SortIndex,
    /// The `.sum`, published by rename.
    Summary,
}

const STEPS: [Step; 6] = [
    Step::Records,
    Step::Checksums,
    Step::TimeIndex,
    Step::Slot,
    Step::SortIndex,
    Step::Summary,
];

/// The positions of `new` that differ from `old`, in file order, restricted
/// to `range`; a position past `old`'s end differs by definition.
fn changed(old: &[u8], new: &[u8], range: std::ops::Range<usize>) -> Vec<usize> {
    range
        .filter(|&i| i < new.len() && old.get(i) != new.get(i))
        .collect()
}

/// `old` with the first `applied` of `positions` taken from `new`, grown to
/// the furthest position written: a sequential write torn after `applied`
/// bytes.
fn torn(old: &[u8], new: &[u8], positions: &[usize], applied: usize) -> Vec<u8> {
    let mut out = old.to_vec();
    for &at in positions.iter().take(applied) {
        if at >= out.len() {
            out.resize(at + 1, 0);
        }
        out[at] = new[at];
    }
    out
}

/// One file's name in the month directory.
fn file_of(step: Step) -> &'static str {
    match step {
        Step::Records | Step::Slot => "2024-06.bin",
        Step::Checksums => "2024-06.crc",
        Step::TimeIndex => "2024-06.tix",
        Step::SortIndex => "2024-06.srt",
        Step::Summary => "2024-06.sum",
    }
}

/// The positions a step writes, given the month before and after the append.
fn writes_of(
    step: Step,
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> Vec<usize> {
    let name = file_of(step);
    let empty = Vec::new();
    let old = before.get(name).unwrap_or(&empty);
    let Some(new) = after.get(name) else {
        return Vec::new();
    };
    let header = usize::try_from(HEADER_LEN).unwrap();
    match step {
        Step::Records => changed(old, new, header.max(old.len())..new.len()),
        Step::Slot => changed(old, new, 0..header),
        // A `.crc` entry is four aligned bytes, and an aligned four-byte write
        // never straddles a sector: the seal tears BETWEEN entries, never
        // inside one. So its writes are whole entries. The tear inside an
        // entry, which only non-atomic storage can make, is its own test
        // below.
        Step::Checksums => {
            let mut entries: Vec<usize> = changed(old, new, 0..new.len().max(old.len()))
                .iter()
                .map(|p| p / 4)
                .collect();
            entries.dedup();
            entries
                .iter()
                .flat_map(|e| e * 4..e * 4 + 4)
                .filter(|&i| i < new.len())
                .collect()
        }
        _ => changed(old, new, 0..new.len().max(old.len())),
    }
}

/// The month after every step before `step` completed and `applied` of
/// `step`'s byte writes landed. A file published by rename is either the old
/// one or the new one, so for those any `applied > 0` is the whole file.
fn crash_state(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
    step: Step,
    applied: usize,
) -> BTreeMap<String, Vec<u8>> {
    let mut out = before.clone();
    for done in STEPS {
        let take = match done.cmp(&step) {
            std::cmp::Ordering::Less => usize::MAX,
            std::cmp::Ordering::Equal => applied,
            std::cmp::Ordering::Greater => 0,
        };
        if take == 0 {
            continue;
        }
        let name = file_of(done);
        let Some(new) = after.get(name) else {
            continue;
        };
        if matches!(done, Step::SortIndex | Step::Summary) {
            out.insert(name.to_owned(), new.clone());
            continue;
        }
        let positions = writes_of(done, before, after);
        let empty = Vec::new();
        let current = out.get(name).unwrap_or(&empty).clone();
        out.insert(name.to_owned(), torn(&current, new, &positions, take));
    }
    out
}

/// The points inside a step of `len` byte writes a crash is placed at.
fn points(step: Step, len: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }
    let mut at = match step {
        Step::SortIndex | Step::Summary => vec![len],
        Step::Records => vec![1, 55, 56, 57, 56 * 7 + 13, len / 2, len - 1, len],
        Step::Slot => (1..=len).collect(),
        Step::Checksums => vec![4, len / 8 * 4, len - 4, len],
        Step::TimeIndex => vec![1, len / 2, len - 1, len],
    };
    at.retain(|&p| p >= 1 && p <= len);
    at.dedup();
    at
}

/// Takes apart the append of `batch` onto a month already holding `held`
/// (in those batches), checks every crash state, and returns the failures.
fn every_crash_point(tag: &str, held: &[Vec<Bar>], batch: &[Bar]) -> Vec<String> {
    let base = Scratch::new(tag);
    let mut file = BarFile::open_or_create(base.root(), month(), SYMBOL).expect("opens");
    for b in held {
        assert!(matches!(file.append(b), Ok(Appended::Committed { .. })));
    }
    let before = snapshot(base.root());
    let n0 = file.records();
    assert!(matches!(file.append(batch), Ok(Appended::Committed { .. })));
    let n1 = file.records();
    // The lock as a dead writer leaves it: its holder note still in place.
    let lock_held = snapshot(base.root());
    drop(file);
    let after = snapshot(base.root());
    let mut expected: Vec<Bar> = held.concat();
    let committed_before = expected.clone();
    expected.extend_from_slice(batch);

    let mut failures = Vec::new();
    let bin = "2024-06.bin";
    let records_only = changed(&before[bin], &after[bin], 0..after[bin].len());
    let header = usize::try_from(HEADER_LEN).unwrap();
    let old_len = before[bin].len();
    if records_only.iter().any(|&p| p >= header && p < old_len) {
        failures.push(format!(
            "{tag}: the append rewrote a byte of the old records region; the crash model is wrong"
        ));
    }

    let mut states = vec![("nothing written".to_owned(), before.clone(), n0)];
    for step in STEPS {
        let len = writes_of(step, &before, &after).len();
        let len = if matches!(step, Step::SortIndex | Step::Summary) && len > 0 {
            1
        } else {
            len
        };
        for p in points(step, len) {
            let committed = if step > Step::Slot || (step == Step::Slot && p == len) {
                n1
            } else {
                n0
            };
            states.push((
                format!("{step:?} {p}/{len}"),
                crash_state(&before, &after, step, p),
                committed,
            ));
        }
    }

    for (label, mut files, committed) in states {
        if let Some(lock) = lock_held.get("2024-06.lock") {
            files.insert("2024-06.lock".to_owned(), lock.clone());
        }
        let run = Scratch::new(&format!("{tag}-state"));
        materialise(run.root(), &files);
        let want = if committed == n1 {
            &expected
        } else {
            &committed_before
        };
        match read_back(run.root()) {
            Ok(bars) if bars == *want => {}
            Ok(bars) => failures.push(format!(
                "{tag} [{label}]: the reader door served {} records, wanted the {} committed",
                bars.len(),
                want.len()
            )),
            Err(why) => failures.push(format!("{tag} [{label}]: the reader door refused: {why}")),
        }
        // The STRICT audit refuses a month with bytes past its commit by
        // design ("non-exact extents"), so a crash state is not asked; the
        // resumed month is, below.
        let resumed = BarFile::open_or_create(run.root(), month(), SYMBOL)
            .map_err(|why| why.to_string())
            .and_then(|mut f| f.append(batch).map_err(|why| why.to_string()));
        match resumed {
            Ok(Appended::Committed { n_valid, .. } | Appended::AlreadyPresent { n_valid, .. })
                if n_valid == n1 => {}
            other => failures.push(format!("{tag} [{label}]: the resume answered {other:?}")),
        }
        let landed = data(&snapshot(run.root()));
        let clean = data(&after);
        if landed != clean {
            let differ: Vec<&String> = clean
                .keys()
                .chain(landed.keys())
                .filter(|k| clean.get(*k) != landed.get(*k))
                .collect();
            failures.push(format!(
                "{tag} [{label}]: the resumed month differs from the clean one in {differ:?}"
            ));
        }
        if let Err(why) = audit(run.root()) {
            failures.push(format!(
                "{tag} [{label}]: the resumed month fails the audit: {why}"
            ));
        }
    }
    failures
}

/// A `.crc` entry torn INSIDE its four bytes, which only storage without
/// atomic sector writes can leave: the tail block's committed records are
/// refused by name (`BlockChecksum`), by the reader and by the resume, and
/// neither writes a byte. Loud, never served: what `CLAUDE.md` section 4 asks
/// of a failure the store cannot repair on its own.
#[test]
fn a_checksum_entry_torn_inside_its_four_bytes_is_refused_by_name_and_never_served() {
    let base = Scratch::new("torn-entry");
    let mut file = BarFile::open_or_create(base.root(), month(), SYMBOL).unwrap();
    for day in 0..3 {
        assert!(matches!(
            file.append(&session(day)),
            Ok(Appended::Committed { .. })
        ));
    }
    let before = snapshot(base.root());
    assert!(matches!(
        file.append(&session(3)),
        Ok(Appended::Committed { .. })
    ));
    drop(file);
    let after = snapshot(base.root());
    let crc = "2024-06.crc";
    let first = changed(&before[crc], &after[crc], 0..after[crc].len())[0];
    let mut state = crash_state(&before, &after, Step::Checksums, 0);
    let mut entry = state[crc].clone();
    entry[first] = after[crc][first];
    state.insert(crc.to_owned(), entry);

    let run = Scratch::new("torn-entry-state");
    materialise(run.root(), &state);
    let left = snapshot(run.root());
    assert!(
        matches!(read_back(run.root()), Err(StoreError::BlockChecksum { .. })),
        "the reader refuses the tail block by name"
    );
    let mut writer = BarFile::open_or_create(run.root(), month(), SYMBOL).unwrap();
    assert!(matches!(
        writer.append(&session(3)),
        Err(StoreError::BlockChecksum { .. })
    ));
    drop(writer);
    assert_eq!(
        data(&snapshot(run.root())),
        data(&left),
        "nothing was written"
    );
}

/// The FIRST append of a month, crashed at every point.
#[test]
fn every_crash_point_of_a_months_first_append_reopens_and_resumes_clean() {
    let failures = every_crash_point("first", &[], &session(0));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The append that crosses the 1,024-bar index threshold (750 -> 1,125),
/// which writes the `.tix`, `.srt` and `.sum` for the first time.
#[test]
fn every_crash_point_of_the_threshold_crossing_append_reopens_and_resumes_clean() {
    let failures = every_crash_point("crossing", &[session(0), session(1)], &session(2));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// An ordinary append above the threshold (1,125 -> 1,500).
#[test]
fn every_crash_point_of_an_indexed_append_reopens_and_resumes_clean() {
    let failures = every_crash_point(
        "indexed",
        &[session(0), session(1), session(2)],
        &session(3),
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A crash that left the next day's records and seal past the commit, then a
/// resume that offers a DIFFERENT batch: the next day after it. Following
/// the commit, it overwrites the dead append's bytes and commits; the day the
/// crash lost is not invented.
#[test]
fn a_resume_that_skips_the_crashed_day_commits_the_next_and_invents_nothing() {
    let base = Scratch::new("skip");
    let mut file = BarFile::open_or_create(base.root(), month(), SYMBOL).unwrap();
    for day in 0..3 {
        assert!(matches!(
            file.append(&session(day)),
            Ok(Appended::Committed { .. })
        ));
    }
    let before = snapshot(base.root());
    assert!(matches!(
        file.append(&session(3)),
        Ok(Appended::Committed { .. })
    ));
    drop(file);
    let after = snapshot(base.root());
    let len = writes_of(Step::TimeIndex, &before, &after).len();
    let state = crash_state(&before, &after, Step::TimeIndex, len);

    let run = Scratch::new("skip-state");
    materialise(run.root(), &state);
    let mut file = BarFile::open_or_create(run.root(), month(), SYMBOL).unwrap();
    assert_eq!(file.records(), 1_125, "the crashed day was never committed");
    let mut next = session(4);
    // The resume's day is a later day, so its stamps follow the commit.
    for (i, bar) in next.iter_mut().enumerate() {
        bar.close = bar.low + i64::try_from(i).unwrap() % 7;
    }
    assert_eq!(
        file.append(&next),
        Ok(Appended::Committed {
            first_index: 1_125,
            n_valid: 1_500
        })
    );
    drop(file);
    let back = read_back(run.root()).expect("the month reads");
    let mut want: Vec<Bar> = (0..3).flat_map(session).collect();
    want.extend_from_slice(&next);
    assert_eq!(back, want, "day 3 is absent, never half-present");
    assert_eq!(audit(run.root()), Ok(()));
}

// ===========================================================================
// 2. A real writer killed by SIGKILL
// ===========================================================================

/// The environment variable that turns this test binary into the writer.
const WRITER: &str = "BRUTEX_EXTREMES_CRASH_WRITER";

/// Twenty sessions in batches of 25 bars: 300 appends, each with its own
/// barriers, so most of the run is spent inside an append.
fn kill_batches() -> Vec<Vec<Bar>> {
    (0..20)
        .flat_map(|day| {
            session(day)
                .chunks(25)
                .map(<[Bar]>::to_vec)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Writes every batch, re-offering the held ones: what a resumed import does.
fn write_all(root: &Path) -> Result<(), StoreError> {
    let mut file = BarFile::open_or_create(root, month(), SYMBOL)?;
    for batch in kill_batches() {
        file.append(&batch)?;
    }
    Ok(())
}

/// The writer process is killed at seven points of its run, each chosen by
/// watching its bar file grow past an eighth of its final size. After every
/// kill the month opens at a whole committed prefix of what was written,
/// passes the checksum audit, and a resume that offers everything again lands
/// the month byte-identical to an uninterrupted run.
#[test]
fn a_writer_killed_at_seven_points_leaves_a_committed_prefix_and_resumes_clean() {
    if let Some(root) = std::env::var_os(WRITER) {
        write_all(Path::new(&root)).expect("the child writes");
        return;
    }
    let clean = Scratch::new("kill-clean");
    write_all(clean.root()).expect("the clean run");
    let want = data(&snapshot(clean.root()));
    let full = want["2024-06.bin"].len();
    let all: Vec<Bar> = kill_batches().concat();

    let mut interrupted = 0;
    let mut failures = Vec::new();
    for eighth in 1..=7_usize {
        let run = Scratch::new(&format!("kill-{eighth}"));
        let mut child = std::process::Command::new(std::env::current_exe().expect("this binary"))
            .args([
                "--exact",
                "a_writer_killed_at_seven_points_leaves_a_committed_prefix_and_resumes_clean",
                "--test-threads=1",
            ])
            .env(WRITER, run.root())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the writer starts");
        let target = HEADER_LEN + u64::try_from((full - 32_768) * eighth / 8).unwrap();
        let bin = month().to_path_buf(run.root());
        let started = std::time::Instant::now();
        loop {
            let grown = fs::metadata(&bin).map_or(0, |m| m.len());
            let done = child.try_wait().expect("the writer's status").is_some();
            if grown >= target || done || started.elapsed() > std::time::Duration::from_mins(1) {
                break;
            }
            std::thread::yield_now();
        }
        child
            .kill()
            .expect("SIGKILL is delivered or the child is gone");
        let status = child.wait().expect("the writer is reaped");
        let back = read_back(run.root());
        match &back {
            Ok(bars) => {
                if bars.len() % 25 != 0 || bars[..] != all[..bars.len()] {
                    failures.push(format!(
                        "kill {eighth}: {} records are not a whole committed prefix",
                        bars.len()
                    ));
                }
                if !status.success() && bars.len() < all.len() {
                    interrupted += 1;
                }
            }
            Err(why) => failures.push(format!("kill {eighth}: the reader door refused: {why}")),
        }
        if let Err(why) = write_all(run.root()) {
            failures.push(format!("kill {eighth}: the resume refused: {why}"));
        }
        if data(&snapshot(run.root())) != want {
            failures.push(format!(
                "kill {eighth}: the resumed month differs from a clean run"
            ));
        }
        if let Err(why) = audit(run.root()) {
            failures.push(format!(
                "kill {eighth}: the resumed month fails the audit: {why}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        interrupted >= 1,
        "no kill landed inside the run, so nothing was tested ({interrupted} of 7)"
    );
}

// ===========================================================================
// 3. Readers during writes
// ===========================================================================

/// A writer that opens, appends one batch and closes, again and again, races
/// two readers that open the month in a loop. Every refusal either side meets
/// is `Locked` by name, every read is a whole committed prefix, and when the
/// writer is done both readers see the whole month.
#[test]
fn two_readers_racing_a_writer_see_only_whole_committed_prefixes() {
    let s = Scratch::new("race");
    let root = s.root().to_path_buf();
    let batches: Vec<Vec<Bar>> = (0..4)
        .flat_map(|d| {
            session(d)
                .chunks(75)
                .map(<[Bar]>::to_vec)
                .collect::<Vec<_>>()
        })
        .collect();
    let all: Vec<Bar> = batches.concat();
    drop(BarFile::open_or_create(&root, month(), SYMBOL).unwrap());
    let writing = AtomicBool::new(true);
    // The writer starts only once both readers run, so the race is real.
    let readers_up = AtomicU64::new(0);

    let (writer_locked, reports) = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut locked = 0_u32;
            while readers_up.load(Ordering::Acquire) < 2 {
                std::thread::yield_now();
            }
            for batch in &batches {
                loop {
                    match BarFile::open_or_create(&root, month(), SYMBOL) {
                        Ok(mut file) => {
                            assert!(matches!(file.append(batch), Ok(Appended::Committed { .. })));
                            break;
                        }
                        Err(StoreError::Locked { .. }) => {
                            locked += 1;
                            std::thread::yield_now();
                        }
                        Err(other) => panic!("the writer met {other}"),
                    }
                }
            }
            writing.store(false, Ordering::Release);
            locked
        });
        let reader = || {
            let mut report = String::new();
            let (mut seen, mut locked) = (0_u32, 0_u32);
            readers_up.fetch_add(1, Ordering::AcqRel);
            while writing.load(Ordering::Acquire) {
                match read_back(&root) {
                    Ok(bars) => {
                        seen += 1;
                        if bars.len() % 75 != 0 || bars[..] != all[..bars.len()] {
                            let _ = writeln!(report, "a partial read of {} records", bars.len());
                        }
                    }
                    Err(StoreError::Locked { .. }) => locked += 1,
                    Err(other) => {
                        let _ = writeln!(report, "a reader met {other}");
                    }
                }
            }
            (report, seen, locked)
        };
        let a = scope.spawn(reader);
        let b = scope.spawn(reader);
        let locked = writer.join().expect("the writer joins");
        (locked, [a.join().expect("joins"), b.join().expect("joins")])
    });
    for (report, _, _) in &reports {
        assert!(report.is_empty(), "{report}");
    }
    let observed: u32 = reports.iter().map(|(_, seen, locked)| seen + locked).sum();
    assert!(observed > 0, "the readers ran while the writer wrote");
    let _ = writer_locked;
    assert_eq!(read_back(&root).expect("the month reads"), all);
    assert_eq!(read_back(&root).expect("and again"), all);
}

/// FINDING (store, a month without its `.lock`): reader/writer exclusion
/// silently does not hold.
///
/// Input: a month of 1,125 bars whose `.lock` file is absent (restored from a
/// copy, or written by a tool that keeps none). A reader opens it through
/// `BarFile::open_existing`, which takes NO lock when the file is missing; a
/// writer then opens the same month through `open_or_create`, which creates
/// the `.lock` and takes it exclusively without meeting the reader.
///
/// Expected: the writer is refused `Locked` while a reader holds the month,
/// exactly as it is when the `.lock` exists (`write.rs`,
/// `a_reader_and_a_writer_of_one_month_refuse_each_other_by_name`).
///
/// Actual: the writer opens and appends 375 bars under the open reader. In
/// this run the reader's own committed prefix still read whole (the store is
/// append-only and the tail proof admits the newer seal), so no wrong bar was
/// observed: the defect is that the exclusion the lock promises is absent and
/// nothing says so. Cause: the reader door's no-lock branch,
/// `crates/store/src/file.rs:2199-2209` (`BarFile::open_existing` when the
/// `.lock` does not exist).
///
/// Proposed fix: take the shared lock on the bar file's own descriptor when
/// no `.lock` exists (a read door may not create the `.lock`, but it can lock
/// what it opened), or refuse a month with no `.lock` by name.
#[test]
#[ignore = "finding: a reader of a month with no .lock file does not exclude a writer (EXTREMES-REPORT.md)"]
fn finding_a_reader_of_a_month_without_its_lock_file_is_invisible_to_a_writer() {
    let s = Scratch::new("nolock");
    let mut file = BarFile::open_or_create(s.root(), month(), SYMBOL).unwrap();
    for day in 0..3 {
        assert!(matches!(
            file.append(&session(day)),
            Ok(Appended::Committed { .. })
        ));
    }
    drop(file);
    let lock = month().with_file(FileKind::Lock).to_path_buf(s.root());
    fs::remove_file(&lock).expect("the lock file goes");

    let reader = BarFile::open_existing(s.root(), month(), SYMBOL).expect("the reader opens");
    let writer = BarFile::open_or_create(s.root(), month(), SYMBOL);
    let refused = matches!(writer, Err(StoreError::Locked { .. }));
    if let Ok(mut w) = writer {
        assert!(matches!(
            w.append(&session(3)),
            Ok(Appended::Committed { .. })
        ));
        drop(w);
    }
    let reads: Result<Vec<Bar>, StoreError> = (0..reader.records())
        .map(|i| reader.read_record(i))
        .collect();
    let held: Vec<Bar> = (0..3).flat_map(session).collect();
    assert_eq!(
        reads.as_ref().ok(),
        Some(&held),
        "the reader's committed prefix reads whole either way"
    );
    assert!(
        refused,
        "a writer must not take a month a reader holds; it took it and appended"
    );
}
