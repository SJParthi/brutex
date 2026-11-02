//! The crash-recovery lines of `sink.rs` that only a damaged disk reaches
//! (BASE-COV-LIBS, D-7320; assurance 29 Oct, COV-11): an unfinished seed that
//! cannot be moved aside, a seed that cannot be opened, the telemetry
//! directory going away while a stream opens, a posting path with no parent.
//! Each is a production refusal, so each is driven by its real input: a
//! read-only parent, a path one byte past the host's limit, a FIFO that parks
//! the writer exactly where the directory is taken from under it.
//!
//! A file of its own so the roll and reaper work in `sink.rs` never meets it;
//! a child of `sink` because `Audit` is private to it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{
    AUDIT_DIMS_FILE, AUDIT_HANDLES, AUDIT_POINTER_BYTES, Audit, AuditHealth, Counters, Opened,
    PICTURE_CODES, RECENT_LINES, audit_current_path, audit_generation_path, audit_serial_path,
    current_path,
};
use crate::level::Level;
use crate::run_dirs::{state_seeding_of, state_stream_of};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The host's `PATH_MAX`, terminator included: a path of this many bytes is
/// refused with `ENAMETOOLONG`, one byte fewer is not.
const PATH_MAX: usize = if cfg!(target_os = "macos") { 1024 } else { 4096 };

/// `O_NONBLOCK` and `ENXIO` of the two hosts this repository runs on; this
/// crate has no `libc` and takes none for a test.
const O_NONBLOCK: i32 = if cfg!(target_os = "macos") { 0x4 } else { 0x800 };
const ENXIO: i32 = 6;

/// How long a test waits for a thread it parked. Far beyond any healthy
/// open; a failure here names a hang, it never passes.
const PARKED: Duration = Duration::from_secs(60);

/// A new real directory, links resolved: `/var` is itself a link on macOS and
/// its expansion would count against the path limit these tests measure.
fn scratch(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "brutex-sink-recovery-{}-{stamp}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

/// A directory made read-only for as long as this lives, so a failed
/// assertion never leaves one behind.
struct ReadOnly<'a>(&'a Path);

impl<'a> ReadOnly<'a> {
    fn new(dir: &'a Path) -> Self {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        Self(dir)
    }
}

impl Drop for ReadOnly<'_> {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// What an earlier start left at the seed's name.
#[derive(Clone, Copy, Debug)]
enum Left {
    /// A directory holding a part of a seed.
    HalfSeed,
    /// A directory holding nothing.
    Empty,
    /// A regular file where a directory belongs.
    File,
    /// A link to nothing: `exists` says no, `symlink_metadata` says yes.
    Dangling,
}

fn leave(left: Left, staging: &Path) {
    match left {
        Left::HalfSeed => {
            std::fs::create_dir_all(staging).unwrap();
            std::fs::write(staging.join("kept"), b"half a seed\n").unwrap();
        }
        Left::Empty => std::fs::create_dir_all(staging).unwrap(),
        Left::File => std::fs::write(staging, b"not a directory\n").unwrap(),
        Left::Dangling => {
            std::os::unix::fs::symlink(staging.with_extension("nowhere"), staging).unwrap();
        }
    }
}

/// **A SEED THAT CANNOT BE MOVED ASIDE REFUSES THE STREAM BY NAME**, whatever
/// form it was left in, keeps what it held, makes no stream and copies
/// nothing; and the next start with the parent writable again keeps the seed,
/// names it and seeds whole.
#[test]
fn an_unfinished_seed_that_cannot_be_moved_aside_is_refused_and_kept() {
    crate::tests::where_permission_binds(
        "sink::recovery_tests::an_unfinished_seed_that_cannot_be_moved_aside_is_refused_and_kept",
        every_form_of_a_seed_that_stays,
    );
}

fn every_form_of_a_seed_that_stays() {
    for left in [Left::HalfSeed, Left::Empty, Left::File, Left::Dangling] {
        let parent = scratch("aside");
        let staging = state_seeding_of(&parent);
        leave(left, &staging);
        // COMBINED with a ring to seed from: it stays exactly as it is.
        std::fs::write(current_path(&parent), b"{\"not a record\n").unwrap();
        {
            let _held = ReadOnly::new(&parent);
            let (state, said) = Audit::open_state(&parent, 5);
            let said = said.unwrap_or_else(|| panic!("{left:?}: the refusal is said"));
            assert!(said.contains("cannot be moved aside"), "{left:?}: {said}");
            assert!(said.contains(&staging.display().to_string()), "{said}");
            assert!(said.contains("cannot create the state stream"), "{said}");
            assert_eq!(state.health.last_error.as_deref(), Some(said.as_str()));
            assert!(state.file.is_none(), "{left:?}: no stream is open");
            assert_eq!(state.health.seeded, 0, "{left:?}: nothing is copied");
        }
        assert!(
            !state_stream_of(&parent).exists(),
            "{left:?}: no half stream"
        );
        assert!(staging.symlink_metadata().is_ok(), "{left:?}: it is kept");
        if matches!(left, Left::HalfSeed) {
            let kept = std::fs::read(staging.join("kept")).unwrap();
            assert_eq!(kept, b"half a seed\n");
        }
        let ring = std::fs::read(current_path(&parent)).unwrap();
        assert_eq!(ring, b"{\"not a record\n", "the ring is never touched");

        // THE DISK IS WRITABLE AGAIN: the same call now heals, and says so.
        let (state, said) = Audit::open_state(&parent, 5);
        assert!(state.file.is_some(), "{left:?}: {said:?}");
        let named = state.health.last_error.clone().unwrap_or_default();
        assert!(
            named.contains("an earlier start stopped while seeding"),
            "{left:?}: {named}"
        );
        assert!(staging.symlink_metadata().is_err(), "{left:?}: moved");
        assert!(state_stream_of(&parent).is_dir(), "{left:?}");
    }
}

/// `parent` at exactly the length that puts the longest path a stream opens
/// inside the seed `over` bytes past the last one the host accepts.
fn parent_with_seed_paths_past_the_limit(name: &str, over: usize) -> PathBuf {
    let root = scratch(name);
    let longest = |parent: &Path| {
        let staging = state_seeding_of(parent);
        let current = audit_current_path(&staging).as_os_str().len();
        let serial = audit_serial_path(&staging).as_os_str().len();
        current.max(serial)
    };
    let tail = longest(&root) - root.as_os_str().len();
    let wanted = PATH_MAX - 1 + over - tail;
    let mut parent = root;
    let mut remain = wanted - parent.as_os_str().len();
    while remain > 200 {
        parent.push("p".repeat(150));
        remain -= 151;
    }
    parent.push("q".repeat(remain - 1));
    std::fs::create_dir_all(&parent).unwrap();
    assert_eq!(longest(&parent), PATH_MAX - 1 + over);
    parent
}

/// **A SEED THAT CANNOT BE OPENED REFUSES THE STREAM BY NAME.** The seed's
/// directory can be made (its own path is short enough) and nothing can be
/// opened inside it: one byte past the host's limit, the seed is refused with
/// the host's words, and nothing takes the stream's name.
#[test]
fn a_seed_one_byte_past_the_path_limit_cannot_be_opened_and_is_refused() {
    let parent = parent_with_seed_paths_past_the_limit("long", 1);
    let staging = state_seeding_of(&parent);
    let (state, said) = Audit::open_state(&parent, 5);
    let said = said.expect("the refusal is said");
    assert!(said.contains("the seed cannot be opened"), "{said}");
    assert!(said.contains("cannot create the state stream"), "{said}");
    assert!(state.file.is_none());
    assert_eq!(state.health.last_error.as_deref(), Some(said.as_str()));
    assert!(staging.is_dir(), "the seed's own directory was made");
    assert!(!state_stream_of(&parent).exists(), "no half stream");
    // A SECOND START over the seed the first one left: its directory moves
    // aside and the same refusal is said, once per start and never a panic.
    let (state, said) = Audit::open_state(&parent, 5);
    assert!(said.unwrap_or_default().contains("cannot be opened"));
    assert!(state.file.is_none());
}

/// **THE LIMIT IS EXACT**: one byte shorter and the seed opens, so the
/// refusal above is the path limit and nothing else.
#[test]
fn a_seed_exactly_at_the_path_limit_opens() {
    let parent = parent_with_seed_paths_past_the_limit("exact", 0);
    let (state, said) = Audit::open_state(&parent, 5);
    let said = said.unwrap_or_default();
    assert!(!said.contains("cannot be opened"), "{said}");
    assert!(state.file.is_some(), "{said}");
    assert!(state_stream_of(&parent).is_dir());
}

/// What a parked open answered.
struct Parked {
    health: AuditHealth,
    said: Option<String>,
    failed: bool,
    open: bool,
}

/// Opens the stream `dir` with its coverage file a FIFO, so the writer parks
/// inside `mark_coverage`'s read; moves `dir` away while it is parked; lets
/// it go on. Answers what it said and where `dir` went.
fn open_while_the_directory_goes_away(name: &str, bytes: &[u8]) -> (Parked, PathBuf, PathBuf) {
    let root = scratch(name);
    let dir = root.join("logs");
    let generation = audit_generation_path(&dir, 0);
    std::fs::create_dir_all(&generation).unwrap();
    std::fs::write(audit_current_path(&dir), bytes).unwrap();
    let fifo = generation.join(AUDIT_DIMS_FILE);
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo: {made}");
    let opening = dir.clone();
    let writer = std::thread::spawn(move || {
        let (mut audit, said, opened) = Audit::open_in(&opening, true);
        Parked {
            health: std::mem::take(&mut audit.health),
            said,
            failed: opened == Opened::Failed,
            open: audit.file.is_some(),
        }
    });
    // A WRITE-ONLY, NON-BLOCKING OPEN of a FIFO succeeds only once a reader
    // is on it, so this is the proof the writer is parked; and while this
    // handle lives its read cannot end. No sleep decides anything here.
    let deadline = Instant::now() + PARKED;
    let parked = loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(&fifo)
        {
            Ok(parked) => break parked,
            Err(e) if e.raw_os_error() == Some(ENXIO) => {
                assert!(Instant::now() < deadline, "the writer never reached the read");
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => panic!("{}: {e}", fifo.display()),
        }
    };
    let moved = root.join("moved");
    std::fs::rename(&dir, &moved).unwrap();
    // The last writer closes: the parked read ends, having read nothing.
    drop(parked);
    while !writer.is_finished() {
        assert!(Instant::now() < deadline, "the parked writer never ended");
        std::thread::sleep(Duration::from_millis(1));
    }
    (writer.join().unwrap(), dir, moved)
}

/// **THE DIRECTORY GOING AWAY WHILE COVERAGE IS MARKED IS REFUSED BY NAME.**
/// An open that lost its directory after its file opened says so in
/// `last_error` and makes nothing at the name: a run's log directory sits
/// under the store's root, so a directory made again here would be a new
/// root on whatever disk held the path (FM-125).
#[test]
fn the_directory_going_away_mid_open_is_named_and_never_made_again() {
    let (parked, dir, moved) = open_while_the_directory_goes_away("gone", b"");
    let said = parked.said.expect("the refusal is said");
    assert!(said.contains("the telemetry directory went away"), "{said}");
    assert_eq!(parked.health.last_error.as_deref(), Some(said.as_str()));
    assert!(!parked.failed, "the file itself opened");
    assert!(parked.open);
    assert!(!dir.exists(), "nothing is made at the name");
    assert!(moved.is_dir());
}

/// COMBINED: the same, over a stream that already holds a line nothing can
/// read any more. The refusal stands, the unknown span is named, and the
/// resume after it makes nothing at the name either.
#[test]
fn the_directory_going_away_over_a_stream_with_lines_names_both() {
    let (parked, dir, _moved) = open_while_the_directory_goes_away("gone-lines", b"garbage\n");
    let said = parked.said.expect("the refusal is said");
    assert!(said.contains("the telemetry directory went away"), "{said}");
    let damage = parked.health.span_damage.expect("the span is named");
    assert!(damage.contains("does not decode"), "{damage}");
    assert!(!parked.failed && parked.open);
    assert!(!dir.exists(), "nothing is made at the name");
}

/// A coverage file that is a directory is a read the host refuses: named,
/// recorded, and the stream still opens: its lines are worth more than the
/// index's knowledge of them.
#[test]
fn a_directory_at_the_coverage_name_is_named_and_the_stream_opens() {
    let dir = scratch("dims-dir");
    let generation = audit_generation_path(&dir, 0);
    std::fs::create_dir_all(generation.join(AUDIT_DIMS_FILE)).unwrap();
    let (audit, said, opened) = Audit::open_in(&dir, true);
    let said = said.expect("the refusal is said");
    assert!(said.contains("cannot read the index coverage"), "{said}");
    assert!(opened != Opened::Failed);
    assert!(audit.file.is_some());
    assert_eq!(audit.health.last_error.as_deref(), Some(said.as_str()));
}

/// **A POSTING PATH WITH NO PARENT MAKES NOTHING AND IS REFUSED BY THE OPEN.**
/// The empty path and the root are the two paths `Path::parent` answers
/// `None` for; the directory step is skipped, and the open refuses them.
#[test]
fn a_posting_path_with_no_parent_is_refused_by_its_open() {
    let dir = scratch("no-parent");
    let (mut audit, said, _) = Audit::open_in(&dir, true);
    assert_eq!(said, None);
    let image = [7_u8; AUDIT_POINTER_BYTES];
    for path in ["", "/"] {
        let why = audit.post(PathBuf::from(path), &image).unwrap_err();
        assert!(why.contains("cannot open"), "{path:?}: {why}");
        assert!(audit.handles.is_empty(), "{path:?}: nothing is held");
    }
}

/// COMBINED: a full handle table and a path with no parent at once. The
/// table is emptied first (it stays bounded), the refusal holds nothing, and
/// the next good posting opens again.
#[test]
fn a_full_handle_table_and_a_path_with_no_parent_stay_bounded() {
    let dir = scratch("full-table");
    let (mut audit, _, _) = Audit::open_in(&dir, true);
    let image = [9_u8; AUDIT_POINTER_BYTES];
    let root = audit_generation_path(&dir, 0);
    for n in 0..AUDIT_HANDLES {
        audit.post(root.join(format!("d{n}")), &image).unwrap();
    }
    assert_eq!(audit.handles.len(), AUDIT_HANDLES);
    assert!(audit.post(PathBuf::new(), &image).is_err());
    assert!(audit.handles.is_empty(), "the bound empties the table");
    let again = root.join("d0");
    audit.post(again.clone(), &image).unwrap();
    assert_eq!(audit.handles.len(), 1);
    let held = std::fs::metadata(&again).unwrap().len();
    assert_eq!(held, 2 * AUDIT_POINTER_BYTES as u64, "an append, not a new file");
}

/// The picture's counters with no ring to date (a direct construction: every
/// real one has a slot per file) count the line and date nothing.
#[test]
fn counters_with_no_ring_slot_still_count_the_line() {
    let mut counters = Counters::with_firsts(std::collections::VecDeque::new());
    counters.count(1, Level::Info, None, b"a line");
    assert_eq!(counters.counted, 1);
    assert!(counters.ring_firsts.is_empty());
    assert_eq!(counters.recent.len(), 1);
}

/// COMBINED: a full live copy with its cursor past the end (a direct write:
/// the walk keeps it in range) replaces nothing, keeps every line it held and
/// comes back in range on the same step.
#[test]
fn a_cursor_past_a_full_live_copy_replaces_nothing_and_comes_back() {
    let mut counters = Counters::with_firsts(std::collections::VecDeque::new());
    counters.recent = vec![b"kept".to_vec(); RECENT_LINES];
    counters.recent_next = RECENT_LINES;
    counters.count(1, Level::Warn, None, b"new");
    assert!(counters.recent.iter().all(|line| line == b"kept"));
    assert_eq!(counters.recent_next, 1 % RECENT_LINES);
    counters.count(2, Level::Warn, None, b"new");
    assert_eq!(counters.recent[1], b"new");
}

/// The code table full of one count: the first least entry gives way, the
/// newcomer inherits its count plus one, and the count of replacements says
/// so; the table's length never grows.
#[test]
fn a_full_code_table_replaces_its_least_entry() {
    let mut counters = Counters::with_firsts(std::collections::VecDeque::new());
    for n in 0..PICTURE_CODES {
        counters.count_code(&format!("code-{n}"));
    }
    counters.count_code("code-0");
    counters.count_code("newcomer");
    assert_eq!(counters.codes.len(), PICTURE_CODES);
    assert_eq!(counters.codes_replaced, 1);
    let held = counters.codes.iter().find(|(code, _)| code == "newcomer");
    assert_eq!(held.map(|(_, count)| *count), Some(2));
    assert!(counters.codes.iter().any(|(code, count)| code == "code-0" && *count == 2));
}
